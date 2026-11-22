import java.io.ByteArrayOutputStream
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
import org.gradle.api.tasks.OutputFiles
import org.gradle.api.tasks.TaskAction
import org.gradle.process.ExecOperations
import org.jetbrains.kotlin.gradle.dsl.JvmTarget
import org.jetbrains.kotlin.gradle.tasks.KotlinCompile

plugins {
    id("com.android.library")
}

abstract class BuildFfiLibrariesTask @Inject constructor(
    private val execOperations: ExecOperations,
) : DefaultTask() {
    @get:InputFiles
    abstract val rustSources: ConfigurableFileCollection

    @get:Internal
    abstract val workspaceRootPath: Property<String>

    @get:Internal
    abstract val jniLibsDirPath: Property<String>

    @get:OutputFiles
    abstract val libraryFiles: ConfigurableFileCollection

    @TaskAction
    fun build() {
        val workspaceRoot = File(workspaceRootPath.get())
        val jniLibsDir = File(jniLibsDirPath.get())
        jniLibsDir.mkdirs()
        execOperations.exec {
            workingDir(workspaceRoot)
            commandLine(
                "cargo",
                "ndk",
                "-t",
                "arm64-v8a",
                "-t",
                "x86_64",
                "-o",
                jniLibsDir.absolutePath,
                "build",
                "--release",
                "--manifest-path",
                workspaceRoot.resolve("Cargo.toml").absolutePath,
                "-p",
                "keynear-ffi",
            )
        }
        libraryFiles.forEach { library ->
            if (!library.exists()) {
                throw GradleException("Missing compiled library at ${library.absolutePath}")
            }
            verifyPageAlignment(library)
        }
    }

    private fun verifyPageAlignment(library: File) {
        val readelf = findOnPath("llvm-readelf")
        if (readelf == null) {
            logger.warn("llvm-readelf unavailable; skipping 16 KB alignment check for ${library.name}")
            return
        }
        val output = ByteArrayOutputStream()
        execOperations.exec {
            commandLine(readelf.absolutePath, "-l", library.absolutePath)
            standardOutput = output
        }
        val alignments = output.toString().lineSequence()
            .filter { it.trimStart().startsWith("LOAD") }
            .mapNotNull {
                it.trim().split(Regex("\\s+")).lastOrNull()
                    ?.removePrefix("0x")
                    ?.toLongOrNull(16)
            }
            .toList()
        when {
            alignments.isEmpty() ->
                logger.warn("No LOAD alignment found for ${library.name}; 16 KB check inconclusive")
            alignments.any { it < 0x4000L } ->
                throw GradleException("${library.name} is not 16 KB page-aligned: $alignments")
        }
    }

    private fun findOnPath(executable: String): File? =
        System.getenv("PATH")
            ?.split(File.pathSeparator)
            ?.asSequence()
            ?.map { File(it, executable) }
            ?.firstOrNull { it.isFile && it.canExecute() }
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
            else -> throw GradleException(
                "Missing compiled keynear-ffi library for arm64-v8a or x86_64 at " +
                    "${arm64Lib.absolutePath} and ${x86Lib.absolutePath}",
            )
        }
        val targetOutDir = outDir.get().asFile
        targetOutDir.deleteRecursively()
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
val arm64Lib = jniLibsDir.file("arm64-v8a/libkeynear_core.so").asFile
val x86Lib = jniLibsDir.file("x86_64/libkeynear_core.so").asFile

val buildFfiLibraries = tasks.register<BuildFfiLibrariesTask>("buildFfiLibraries") {
    description = "Cross-compile keynear-ffi with cargo ndk for arm64-v8a and x86_64"
    rustSources.from(
        fileTree(workspaceRoot.resolve("crates/keynear-ffi/src")),
        fileTree(workspaceRoot.resolve("crates/keynear-protocol/src")),
    )
    workspaceRootPath.set(workspaceRoot.absolutePath)
    jniLibsDirPath.set(jniLibsDir.asFile.absolutePath)
    libraryFiles.from(arm64Lib, x86Lib)
}

val generateUniffiBindings = tasks.register<GenerateUniffiBindingsTask>("generateUniffiBindings") {
    dependsOn(buildFfiLibraries)
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
        consumerProguardFiles("src/main/consumer-rules.pro")
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
            kotlin.directories.add(layout.buildDirectory.dir("generated/source/uniffi").get().asFile.absolutePath)
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

tasks.matching { it.name == "preBuild" || it.name.startsWith("assemble") }.configureEach {
    dependsOn(generateUniffiBindings)
}

dependencies {
    api("net.java.dev.jna:jna:5.19.1@aar")
}

