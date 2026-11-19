import org.gradle.api.GradleException
import org.gradle.api.tasks.Exec
import org.jetbrains.kotlin.gradle.dsl.JvmTarget
import org.jetbrains.kotlin.gradle.tasks.KotlinCompile

plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
}

val workspaceRoot = rootProject.projectDir.parentFile
val generatedUniffiDir = layout.buildDirectory.dir("generated/source/uniffi")
val jniLibsDir = layout.projectDirectory.dir("src/main/jniLibs")

val generateUniffiBindings by tasks.registering(Exec::class) {
    val arm64Lib = jniLibsDir.file("arm64-v8a/libkeynear_core.so").asFile
    val x86Lib = jniLibsDir.file("x86_64/libkeynear_core.so").asFile
    val outDir = generatedUniffiDir.get().asFile
    inputs.file(arm64Lib).optional()
    inputs.files(fileTree(workspaceRoot.resolve("crates/keynear-ffi/src")))
    inputs.files(fileTree(workspaceRoot.resolve("crates/keynear-protocol/src")))
    outputs.dir(generatedUniffiDir)
    workingDir(workspaceRoot)
    doFirst {
        val libFile = when {
            arm64Lib.exists() -> arm64Lib
            x86Lib.exists() -> x86Lib
            else -> throw GradleException("Missing compiled libkeynear_core.so under ${jniLibsDir.asFile}; run cargo ndk first.")
        }
        outDir.mkdirs()
        commandLine(
            "cargo",
            "run",
            "--manifest-path",
            workspaceRoot.resolve("Cargo.toml").absolutePath,
            "-p",
            "keynear-ffi",
            "--features",
            "bindgen-cli",
            "--bin",
            "uniffi-bindgen",
            "--",
            "generate",
            "--library",
            libFile.absolutePath,
            "--language",
            "kotlin",
            "--out-dir",
            outDir.absolutePath,
            "--no-format",
        )
    }
}

android {
    namespace = "org.keynear.coreffi"
    compileSdk = 36
    buildToolsVersion = "36.0.0"

    defaultConfig {
        minSdk = 31
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_21
        targetCompatibility = JavaVersion.VERSION_21
    }

    packaging {
        jniLibs {
            useLegacyPackaging = false
        }
    }

    sourceSets {
        getByName("main") {
            java.srcDir(generatedUniffiDir)
        }
    }
}

kotlin {
    compilerOptions {
        jvmTarget.set(JvmTarget.JVM_21)
    }
}

tasks.withType<KotlinCompile>().configureEach {
    dependsOn(generateUniffiBindings)
}

tasks.matching { it.name == "preBuild" }.configureEach {
    dependsOn(generateUniffiBindings)
}

dependencies {
    api("net.java.dev.jna:jna:5.19.1@aar")
}

