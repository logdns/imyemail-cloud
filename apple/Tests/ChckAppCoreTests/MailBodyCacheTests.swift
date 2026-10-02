import ChckAppCore
import XCTest

@MainActor
final class MailBodyCacheTests: XCTestCase {
    func testMemoryAndPersistentHitsDoNotFetchOrMarkRead() async throws {
        let engine = BodyCacheEngine()
        await engine.setCached(MailBody(text: "stored", html: "", id: "alice:INBOX:1"), for: "alice:INBOX:1")
        let cache = MailBodyCache(engine: engine)
        let first = try await cache.load("alice:INBOX:1")
        let second = try await cache.load("alice:INBOX:1")
        XCTAssertEqual(first.text, "stored")
        XCTAssertEqual(second.text, "stored")
        let counts = await engine.counts()
        XCTAssertEqual(counts.cached, 1)
        XCTAssertEqual(counts.fetched, 0)
        XCTAssertEqual(counts.flags, 0)
    }

    func testConcurrentLoadsShareOneFetch() async throws {
        let engine = BodyCacheEngine(delayed: true)
        let cache = MailBodyCache(engine: engine)
        let first = Task { try await cache.load("alice:INBOX:1") }
        await engine.waitForRequest(1)
        var secondStarted = false
        let second = Task {
            secondStarted = true
            return try await cache.load("alice:INBOX:1")
        }
        while !secondStarted { await Task.yield() }
        await engine.finish(1, text: "shared")
        let results = try await (first.value, second.value)
        XCTAssertEqual(results.0.text, "shared")
        XCTAssertEqual(results.1.text, "shared")
        let counts = await engine.counts()
        XCTAssertEqual(counts.cached, 1)
        XCTAssertEqual(counts.fetched, 1)
        XCTAssertEqual(counts.flags, 0)
    }

    func testAccountIdentityAndLeastRecentlyUsedEviction() async throws {
        let engine = BodyCacheEngine()
        let cache = MailBodyCache(engine: engine, capacity: 2)
        _ = try await cache.load("alice:INBOX:1")
        _ = try await cache.load("bob:INBOX:1")
        XCTAssertEqual(cache.peek("alice:INBOX:1")?.id, "alice:INBOX:1")
        _ = try await cache.load("alice:INBOX:2")
        XCTAssertNil(cache.peek("bob:INBOX:1"))
        XCTAssertNotNil(cache.peek("alice:INBOX:1"))
        XCTAssertNotNil(cache.peek("alice:INBOX:2"))
        _ = try await cache.load("bob:INBOX:1")
        let counts = await engine.counts()
        XCTAssertEqual(counts.fetched, 4)
    }

    func testByteBudgetCountsUTF8AndSkipsOversizedBody() async throws {
        let engine = BodyCacheEngine()
        await engine.setCached(MailBody(text: "你好", html: ""), for: "a")
        await engine.setCached(MailBody(text: "世界", html: ""), for: "b")
        await engine.setCached(MailBody(text: String(repeating: "x", count: 20), html: ""), for: "large")
        let cache = MailBodyCache(engine: engine, capacity: 10, byteLimit: 10)
        _ = try await cache.load("a")
        _ = try await cache.load("b")
        XCTAssertNil(cache.peek("a"))
        XCTAssertNotNil(cache.peek("b"))
        let oversized = try await cache.load("large")
        XCTAssertEqual(oversized.text.count, 20)
        XCTAssertNil(cache.peek("large"))
        XCTAssertNotNil(cache.peek("b"))
    }

    func testFailureIsNotCachedAndCanRetry() async throws {
        let engine = BodyCacheEngine()
        await engine.failNextFetch()
        let cache = MailBodyCache(engine: engine)
        do {
            _ = try await cache.load("alice:INBOX:1")
            XCTFail("The first fetch should fail")
        } catch {}
        XCTAssertNil(cache.peek("alice:INBOX:1"))
        let body = try await cache.load("alice:INBOX:1")
        XCTAssertEqual(body.id, "alice:INBOX:1")
        let counts = await engine.counts()
        XCTAssertEqual(counts.fetched, 2)
    }

    func testRemoveAllPreventsLateResponseFromRestoringCache() async throws {
        let engine = BodyCacheEngine(delayed: true)
        let cache = MailBodyCache(engine: engine)
        let pending = Task { try await cache.load("alice:INBOX:1") }
        await engine.waitForRequest(1)
        cache.removeAll()
        // The fake deliberately ignores cancellation, like a native operation already in progress.
        await engine.finish(1, text: "late")
        _ = try await pending.value
        XCTAssertNil(cache.peek("alice:INBOX:1"))
    }

    func testRemoveDoesNotLetOldResponseOverwriteReplacement() async throws {
        let engine = BodyCacheEngine(delayed: true)
        let cache = MailBodyCache(engine: engine)
        let old = Task { try await cache.load("alice:INBOX:1") }
        await engine.waitForRequest(1)
        cache.remove("alice:INBOX:1")
        let replacement = Task { try await cache.load("alice:INBOX:1") }
        await engine.waitForRequest(2)
        await engine.finish(2, text: "new")
        _ = try await replacement.value
        await engine.finish(1, text: "old")
        _ = try await old.value
        XCTAssertEqual(cache.peek("alice:INBOX:1")?.text, "new")
    }

    func testRemoveAndRemoveAllClearStoredBodies() async throws {
        let cache = MailBodyCache(engine: BodyCacheEngine())
        _ = try await cache.load("alice:INBOX:1")
        _ = try await cache.load("bob:INBOX:1")
        cache.remove("alice:INBOX:1")
        XCTAssertNil(cache.peek("alice:INBOX:1"))
        XCTAssertNotNil(cache.peek("bob:INBOX:1"))
        cache.removeAll()
        XCTAssertNil(cache.peek("bob:INBOX:1"))
    }
}

private actor BodyCacheEngine: MailEngineClient {
    private let delayed: Bool
    private var cachedBodies: [String: MailBody] = [:]
    private var cachedCount = 0
    private var fetchedCount = 0
    private var flagCount = 0
    private var failNext = false
    private var pending: [Int: CheckedContinuation<MailBody, Error>] = [:]
    private var requestedIDs: [Int: String] = [:]
    private var waiters: [Int: [CheckedContinuation<Void, Never>]] = [:]

    init(delayed: Bool = false) { self.delayed = delayed }

    func setCached(_ body: MailBody, for id: String) { cachedBodies[id] = body }
    func failNextFetch() { failNext = true }
    func counts() -> (cached: Int, fetched: Int, flags: Int) { (cachedCount, fetchedCount, flagCount) }

    func waitForRequest(_ number: Int) async {
        if pending[number] != nil { return }
        await withCheckedContinuation { waiters[number, default: []].append($0) }
    }

    func finish(_ number: Int, text: String) {
        let id = requestedIDs.removeValue(forKey: number) ?? ""
        pending.removeValue(forKey: number)?.resume(returning: MailBody(text: text, html: "", id: id))
    }

    func cachedBody(messageId: String) async throws -> MailBody? {
        cachedCount += 1
        return cachedBodies[messageId]
    }

    func body(messageId: String) async throws -> MailBody {
        fetchedCount += 1
        if failNext {
            failNext = false
            throw CliError.failed("temporary body fetch failure")
        }
        if delayed {
            let number = fetchedCount
            requestedIDs[number] = messageId
            return try await withCheckedThrowingContinuation { continuation in
                pending[number] = continuation
                for waiter in waiters.removeValue(forKey: number) ?? [] { waiter.resume() }
            }
        }
        return MailBody(text: messageId, html: "", id: messageId)
    }

    func setFlags(messageId: String, flags: MailFlags) async throws { flagCount += 1 }
    func accounts() async throws -> [MailAccount] { [] }
    func providers() async throws -> [MailProvider] { [] }
    func folders(accountId: String) async throws -> [MailFolder] { [] }
    func sync(folderId: String) async throws {}
    func messages(folderId: String) async throws -> [MailRow] { [] }
    func addAccount(email: String, host: String?, port: UInt16?, password: String?, insecure: Bool, displayName: String?) async throws -> MailAccount { throw CliError.failed("unused") }
    func send(accountId: String, to: String, cc: String, bcc: String, subject: String, body: String, html: String? = nil) async throws -> String { throw CliError.failed("unused") }
}
