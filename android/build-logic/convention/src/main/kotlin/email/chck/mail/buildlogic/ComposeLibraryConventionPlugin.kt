package email.imy.cloud.buildlogic

import com.android.build.api.dsl.LibraryExtension
import org.gradle.api.Plugin
import org.gradle.api.Project
import org.gradle.kotlin.dsl.configure
import org.gradle.kotlin.dsl.dependencies

class ComposeLibraryConventionPlugin : Plugin<Project> {
    override fun apply(target: Project) = with(target) {
        pluginManager.apply("email.imy.cloud.android.library")
        pluginManager.apply("org.jetbrains.kotlin.plugin.compose")
        extensions.configure<LibraryExtension> {
            buildFeatures.compose = true
        }
        val bom = "androidx.compose:compose-bom:2024.12.01"
        dependencies {
            add("implementation", platform(bom))
            add("implementation", "androidx.compose.ui:ui")
            add("implementation", "androidx.compose.material3:material3")
            add("implementation", "androidx.compose.material:material-icons-extended")
            add("implementation", "androidx.compose.ui:ui-tooling-preview")
            add("debugImplementation", "androidx.compose.ui:ui-tooling")
        }
    }
}
