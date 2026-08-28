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

// IMPORTANT: Imports in Kotlin source are `androidx.compose.*` because Compose
// Multiplatform (JetBrains) shares the same API surface with Jetpack Compose.
// The resolved artifacts are `org.jetbrains.compose.*` and include desktop
// JVM targets for Windows, macOS and Linux. We do NOT depend on Android-only
// androidx.compose artifacts.

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
                implementation(compose.materialIconsExtended)
                implementation("io.ktor:ktor-client-cio:3.0.3")

                // ONNX Runtime for local text/vision/translator inference.
                implementation("com.microsoft.onnxruntime:onnxruntime:1.19.0")

                // Global hotkeys on macOS/Windows/Linux. Requires Accessibility
                // permission on macOS when running packaged apps.
                implementation("com.github.kwhat:jnativehook:2.2.2")

                // AWT SystemTray and Clipboard are built into the JDK.
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

            // Extra native binaries (mirage-daemon, mirage CLI) are copied into
            // this directory by the packaging scripts before running Gradle.
            appResourcesRootDir.set(project.file("package-resources"))

            modules("java.instrument", "java.sql", "jdk.unsupported")

            macOS {
                bundleID = "com.mirage.desktop"
                packageName = "Mirage"
            }

            windows {
                shortcut = true
                menuGroup = "Mirage"
                dirChooser = true
                perUserInstall = true
            }

            linux {
                menuGroup = "Mirage"
            }
        }
    }
}
