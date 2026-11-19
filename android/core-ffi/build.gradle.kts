import java.io.File
import javax.inject.Inject
import org.gradle.api.DefaultTask
import org.gradle.api.GradleException
import org.gradle.api.file.ConfigurableFileCollection
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.provider.Property
import org.gradle.api.tasks.InputFiles
import org.gradle.api.tasks.Internal
import org.gradle.api.tasks.Optional
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.TaskAction
import org.gradle.process.ExecOperations
import org.jetbrains.kotlin.gradle.dsl.JvmTarget
import org.jetbrains.kotlin.gradle.tasks.KotlinCompile

plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
}

abstract class GenerateUniffiBindingsTask @Inject constructor(
    private val execOperations: ExecOperations,
) : DefaultTask() {
    @get:InputFiles
    @get:Optional
    abstract val jniLibFiles: ConfigurableFileCollection

    @get:InputFiles
    abstract val rustSources: ConfigurableFileCollection

    @get:Internal
    abstract val arm64LibPath: Property<String>

    @get:Internal
    abstract val x86LibPath: Property<String>

    @get:Internal
    abstract val workspaceRootPath: Property<String>

    @get:OutputDirectory
    abstract val outDir: DirectoryProperty

    @TaskAction
    fun generate() {
        val workspaceRoot = File(workspaceRootPath.get())
        val manifestPath = workspaceRoot.resolve("Cargo.toml").absolutePath
        val arm64Lib = File(arm64LibPath.get())
        val x86Lib = File(x86LibPath.get())
        val libFile = when {
            arm64Lib.exists() -> arm64Lib
            x86Lib.exists() -> x86Lib
            else -> {
                execOperations.exec {
                    workingDir(workspaceRoot)
                    commandLine(
                        "cargo",
                        "build",
                        "--manifest-path",
                        manifestPath,
                        "-p",
                        "keynear-ffi",
                    )
                }
                val osName = System.getProperty("os.name").lowercase()
                val ext = when {
                    osName.contains("mac") -> "dylib"
                    osName.contains("win") -> "dll"
                    else -> "so"
                }
                val prefix = if (osName.contains("win")) "" else "lib"
                val hostLib = workspaceRoot.resolve("target/debug/${prefix}keynear_core.$ext")
                if (!hostLib.exists()) {
                    throw GradleException("Missing compiled library at ${hostLib.absolutePath}")
                }
                hostLib
            }
        }
        val targetOutDir = outDir.get().asFile
        targetOutDir.mkdirs()
        execOperations.exec {
            workingDir(workspaceRoot)
            commandLine(
                "cargo",
                "run",
                "--manifest-path",
                manifestPath,
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
                targetOutDir.absolutePath,
                "--no-format",
            )
        }
    }
}

val workspaceRoot = rootProject.projectDir.parentFile
val generatedUniffiDir = layout.buildDirectory.dir("generated/source/uniffi")
val jniLibsDir = layout.projectDirectory.dir("src/main/jniLibs")

val generateUniffiBindings by tasks.registering(GenerateUniffiBindingsTask::class) {
    val arm64Lib = jniLibsDir.file("arm64-v8a/libkeynear_core.so").asFile
    val x86Lib = jniLibsDir.file("x86_64/libkeynear_core.so").asFile
    jniLibFiles.from(arm64Lib, x86Lib)
    rustSources.from(
        fileTree(workspaceRoot.resolve("crates/keynear-ffi/src")),
        fileTree(workspaceRoot.resolve("crates/keynear-protocol/src")),
    )
    arm64LibPath.set(arm64Lib.absolutePath)
    x86LibPath.set(x86Lib.absolutePath)
    workspaceRootPath.set(workspaceRoot.absolutePath)
    outDir.set(generatedUniffiDir)
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

    lint {
        checkGeneratedSources = false
        disable += "NewApi"
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

