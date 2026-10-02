plugins {
    id("email.imy.cloud.compose.library")
}
android {
    namespace = "email.imy.cloud.feature.messagelist"
}
dependencies {
    implementation(project(":core:common"))
    implementation(project(":core:designsystem"))
}
