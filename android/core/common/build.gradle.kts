plugins {
    id("java-library")
    alias(libs.plugins.kotlin.jvm)
}
java {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
}
kotlin {
    jvmToolchain(17)
}
dependencies {
    implementation("org.commonmark:commonmark:0.24.0")
    implementation("org.commonmark:commonmark-ext-gfm-tables:0.24.0")
    implementation("org.commonmark:commonmark-ext-gfm-strikethrough:0.24.0")
    implementation(libs.kotlinx.coroutines.core)
    testImplementation(kotlin("test"))
    testImplementation(libs.kotlinx.coroutines.test)
}
tasks.test {
    useJUnitPlatform()
}
