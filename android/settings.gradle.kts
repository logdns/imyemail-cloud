pluginManagement {
    includeBuild("build-logic")
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}
dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}
rootProject.name = "android"
include(":app")
include(":core:common")
include(":core:data")
include(":core:designsystem")
include(":core:chckcore")
include(":feature:onboarding")
include(":feature:mailbox")
include(":feature:messagelist")
include(":feature:thread")
include(":feature:compose")
include(":feature:search")
include(":feature:contacts")
include(":feature:rules")
include(":feature:settings")
