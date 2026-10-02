plugins {
    `kotlin-dsl`
}
group = "email.imy.cloud.buildlogic"
java {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
}
dependencies {
    implementation("com.android.tools.build:gradle:8.7.3")
    implementation("org.jetbrains.kotlin:kotlin-gradle-plugin:2.1.0")
    implementation("org.jetbrains.kotlin:compose-compiler-gradle-plugin:2.1.0")
}
gradlePlugin {
    plugins {
        register("androidLibrary") {
            id = "email.imy.cloud.android.library"
            implementationClass = "email.imy.cloud.buildlogic.AndroidLibraryConventionPlugin"
        }
        register("composeLibrary") {
            id = "email.imy.cloud.compose.library"
            implementationClass = "email.imy.cloud.buildlogic.ComposeLibraryConventionPlugin"
        }
    }
}
