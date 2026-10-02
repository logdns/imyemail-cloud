plugins {
    id("email.imy.cloud.android.library")
}
android {
    namespace = "email.imy.cloud.chckcore"
    testOptions.unitTests.all { it.useJUnitPlatform() }
}
dependencies {
    api(project(":core:common"))
    api(project(":core:data"))
    implementation(libs.androidx.security.crypto)
    testImplementation(kotlin("test"))
    testImplementation(libs.kotlinx.coroutines.test)
    testImplementation(libs.kotlinx.coroutines.core)
}
