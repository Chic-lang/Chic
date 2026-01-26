plugins {
    kotlin("jvm") version "1.9.25"
    id("org.jetbrains.intellij") version "1.17.4"
}

group = "com.chic"
version = "0.0.1"

repositories {
    mavenCentral()
}

intellij {
    version.set("2023.3.6")
    type.set("IC")
}

tasks {
    patchPluginXml {
        sinceBuild.set("233")
        untilBuild.set("243.*")
    }
}

