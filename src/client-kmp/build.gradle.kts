import org.jetbrains.compose.desktop.application.dsl.TargetFormat

plugins {
    kotlin("multiplatform") version "2.1.0"
    id("org.jetbrains.compose") version "1.7.3"
    id("org.jetbrains.kotlin.plugin.compose") version "2.1.0"
    id("org.jetbrains.kotlin.plugin.serialization") version "2.1.0"
}

repositories {
    mavenCentral()
    google()
    gradlePluginPortal()
    maven("https://maven.pkg.jetbrains.space/public/p/compose/dev")
}

kotlin {
    jvm()

    sourceSets {
        val commonMain by getting {
            dependencies {
                implementation(compose.runtime)
                implementation(compose.foundation)
                implementation(compose.material3)
                implementation(compose.ui)

                implementation("io.ktor:ktor-client-core:3.0.3")
                implementation("io.ktor:ktor-client-content-negotiation:3.0.3")
                implementation("io.ktor:ktor-serialization-kotlinx-json:3.0.3")
                implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.7.3")

                // LanceDB JVM client is not yet available on Maven Central under
                // com.github.lancedb or com.lancedb. Native/JNI integration for the
                // desktop targets is pending and will be added in T3.4.
            }
        }

        val jvmMain by getting {
            dependencies {
                implementation(compose.desktop.currentOs)
                implementation("io.ktor:ktor-client-cio:3.0.3")
            }
        }

        val commonTest by getting {
            dependencies {
                implementation(kotlin("test"))
                implementation("io.ktor:ktor-client-mock:3.0.3")
                implementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:1.9.0")
            }
        }

        val jvmTest by getting {
            dependencies {
                implementation(kotlin("test"))
                implementation("io.ktor:ktor-client-cio:3.0.3")
                implementation("org.jetbrains.compose.ui:ui-test:1.7.3")
                implementation(compose.desktop.currentOs)
            }
        }
    }
}

compose.desktop {
    application {
        mainClass = "mirage.desktop.MainKt"
        nativeDistributions {
            targetFormats(TargetFormat.Dmg, TargetFormat.Msi, TargetFormat.Deb)
            packageName = "Mirage"
            packageVersion = "1.0.0"
        }
    }
}
