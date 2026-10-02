plugins {
    id("email.imy.cloud.compose.library")
}
android {
    namespace = "email.imy.cloud.feature.thread"
}
dependencies {
    implementation(project(":core:common"))
    implementation(project(":core:designsystem"))
    testImplementation(kotlin("test"))
}
