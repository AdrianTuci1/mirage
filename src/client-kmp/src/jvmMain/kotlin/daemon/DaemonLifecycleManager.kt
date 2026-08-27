package mirage.daemon

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import java.io.File
import java.nio.file.Files
import java.nio.file.Paths

private const val READY_TIMEOUT_MS = 10_000L
private const val PING_RETRY_MS = 250L

/**
 * Manages the lifecycle of the local Mirage daemon.
 *
 * The daemon is not a persistent OS service; it is spawned on demand by the
 * GUI or by the CLI, lives while needed, and is shut down when the app quits.
 *
 * The manager looks for the daemon executable next to the running application,
 * on the system PATH, or at the path given by `MIRAGE_DAEMON_EXE`.
 */
class DaemonLifecycleManager(
    private val socketPath: String,
    private val dataDir: String,
    private val modelsDir: String,
    private val downloadsDir: String
) {
    private var process: Process? = null

    /**
     * Ensure the daemon is running and responding on [socketPath].
     *
     * If it is already running, returns immediately. Otherwise it spawns the
     * bundled daemon binary and waits for the socket to accept a ping.
     */
    suspend fun ensureRunning(): DaemonClient = withContext(Dispatchers.IO) {
        val client = DaemonClient(socketPath)
        if (client.ping()) {
            return@withContext client
        }

        spawnDaemon()
        waitForSocket(client)
        client
    }

    /**
     * Stop the daemon if this manager started it.
     */
    suspend fun stop() = withContext(Dispatchers.IO) {
        process?.let { p ->
            if (p.isAlive) {
                p.destroy()
                try {
                    if (!p.waitFor(5, java.util.concurrent.TimeUnit.SECONDS)) {
                        p.destroyForcibly()
                    }
                } catch (_: InterruptedException) {
                    p.destroyForcibly()
                }
            }
        }
        process = null
        // Clean up the stale socket file if it exists.
        try {
            Files.deleteIfExists(Paths.get(socketPath))
        } catch (_: Exception) {
        }
    }

    private fun spawnDaemon() {
        val exe = resolveDaemonExe()
            ?: throw DaemonClient.DaemonException("mirage-daemon executable not found. Set MIRAGE_DAEMON_EXE or ensure it is on PATH.")

        val parentDir = File(socketPath).parentFile
        if (parentDir != null && !parentDir.exists()) {
            parentDir.mkdirs()
        }

        val builder = ProcessBuilder(
            exe.absolutePath,
            "--socket-path", socketPath,
            "--data-dir", dataDir,
            "--models-dir", modelsDir,
            "--downloads-dir", downloadsDir
        )
        builder.inheritIO()
        process = builder.start()
    }

    private suspend fun waitForSocket(client: DaemonClient) {
        val start = System.currentTimeMillis()
        while (System.currentTimeMillis() - start < READY_TIMEOUT_MS) {
            if (client.ping()) return
            delay(PING_RETRY_MS)
        }
        throw DaemonClient.DaemonException("daemon did not become ready within ${READY_TIMEOUT_MS}ms")
    }

    private fun resolveDaemonExe(): File? {
        System.getenv("MIRAGE_DAEMON_EXE")?.let {
            val file = File(it)
            if (file.exists() && file.canExecute()) return file
        }

        // Look next to the running app / JVM.
        val candidates = mutableListOf<File>()

        // Compose Desktop / packaged app resources.
        System.getProperty("compose.application.resources.dir")?.let { dir ->
            candidates.add(File(dir, "mirage-daemon"))
        }

        // Directory of the JVM executable / app bundle.
        System.getProperty("java.home")?.let { home ->
            val homeFile = File(home)
            candidates.add(File(homeFile.parentFile, "mirage-daemon"))
            candidates.add(File(homeFile.parentFile?.parentFile, "MacOS/mirage-daemon"))
        }

        // Working directory.
        candidates.add(File("mirage-daemon"))

        // PATH.
        System.getenv("PATH")?.split(File.pathSeparator)?.forEach { entry ->
            candidates.add(File(entry, "mirage-daemon"))
        }

        candidates.firstOrNull { it.exists() && it.canExecute() }?.let { return it }

        return null
    }
}
