import Foundation

/// A process-local body cache. Keys must be complete MailRow IDs, including account identity.
@MainActor
public final class MailBodyCache {
    private struct Entry {
        let body: MailBody
        let bytes: Int
    }

    private struct Request {
        let generation: UInt64
        let task: Task<MailBody, Error>
    }

    private let engine: any MailEngineClient
    private let capacity: Int
    private let byteLimit: Int
    private var entries: [String: Entry] = [:]
    private var recency: [String] = []
    private var bytes = 0
    private var requests: [String: Request] = [:]
    private var generation: UInt64 = 0

    public init(engine: any MailEngineClient, capacity: Int = 24, byteLimit: Int = 8 * 1024 * 1024) {
        self.engine = engine
        self.capacity = max(0, capacity)
        self.byteLimit = max(0, byteLimit)
    }

    public func peek(_ id: String) -> MailBody? {
        guard let entry = entries[id] else { return nil }
        touch(id)
        return entry.body
    }

    /// Reads cached or fetched content without changing read flags.
    public func load(_ id: String) async throws -> MailBody {
        if let body = peek(id) { return body }
        let request: Request
        if let existing = requests[id] {
            request = existing
        } else {
            generation &+= 1
            let engine = engine
            let task = Task<MailBody, Error> {
                if let cached = try await engine.cachedBody(messageId: id) { return cached }
                try Task.checkCancellation()
                return try await engine.body(messageId: id)
            }
            request = Request(generation: generation, task: task)
            requests[id] = request
        }
        do {
            let body = try await request.task.value
            // Removal may have started a replacement request while this one was suspended.
            if requests[id]?.generation == request.generation {
                requests.removeValue(forKey: id)
                insert(body, for: id)
            }
            return body
        } catch {
            if requests[id]?.generation == request.generation {
                requests.removeValue(forKey: id)
            }
            throw error
        }
    }

    public func remove(_ id: String) {
        evict(id)
        requests.removeValue(forKey: id)?.task.cancel()
    }

    public func removeAll() {
        entries.removeAll()
        recency.removeAll()
        bytes = 0
        for request in requests.values { request.task.cancel() }
        requests.removeAll()
    }

    private func insert(_ body: MailBody, for id: String) {
        // Bound retained string payload; this is not a measurement of allocator overhead.
        let cost = id.utf8.count + body.id.utf8.count + body.text.utf8.count + body.html.utf8.count
        guard capacity > 0, byteLimit > 0, cost <= byteLimit else { return }
        evict(id)
        while entries.count >= capacity || bytes > byteLimit - cost {
            guard let oldest = recency.first else { break }
            evict(oldest)
        }
        entries[id] = Entry(body: body, bytes: cost)
        bytes += cost
        touch(id)
    }

    private func touch(_ id: String) {
        recency.removeAll { $0 == id }
        recency.append(id)
    }

    private func evict(_ id: String) {
        if let entry = entries.removeValue(forKey: id) { bytes -= entry.bytes }
        recency.removeAll { $0 == id }
    }
}
