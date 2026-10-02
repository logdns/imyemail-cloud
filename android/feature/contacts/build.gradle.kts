plugins {
    id("email.imy.cloud.compose.library")
}
android {
    namespace = "email.imy.cloud.feature.contacts"
}
dependencies {
    implementation(project(":core:common"))
    implementation(project(":core:designsystem"))
}
