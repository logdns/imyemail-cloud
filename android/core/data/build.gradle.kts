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
    api(project(":core:common"))
    implementation(libs.kotlinx.coroutines.core)
    implementation(libs.kotlinx.serialization.json)
    testImplementation(kotlin("test"))
    testImplementation(libs.kotlinx.coroutines.test)
}
tasks.test {
    useJUnitPlatform()
}
