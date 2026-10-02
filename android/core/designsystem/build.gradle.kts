plugins {
    id("email.imy.cloud.compose.library")
}
android {
    namespace = "email.imy.cloud.design"
}
dependencies {
    implementation(project(":core:common"))
}
