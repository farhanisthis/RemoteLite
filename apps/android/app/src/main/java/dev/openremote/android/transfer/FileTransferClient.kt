package dev.openremote.android.transfer

import org.json.JSONObject
import java.io.File
import java.io.FileInputStream
import java.io.FileOutputStream
import java.io.RandomAccessFile
import java.security.MessageDigest
import java.util.UUID

/**
 * Announcement of an upcoming file transfer.
 */
data class TransferAnnounce(
    val transferId: String,
    val fileName: String,
    val fileSize: Long,
    val sha256: String,
    val direction: String
) {
    companion object {
        fun fromJson(jsonStr: String): TransferAnnounce {
            val json = JSONObject(jsonStr)
            return TransferAnnounce(
                transferId = json.getString("transfer_id"),
                fileName = json.getString("file_name"),
                fileSize = json.getLong("file_size"),
                sha256 = json.getString("sha256"),
                direction = json.getString("direction")
            )
        }
    }

    fun toJson(): String {
        val json = JSONObject()
        json.put("type", "announce")
        json.put("transfer_id", transferId)
        json.put("file_name", fileName)
        json.put("file_size", fileSize)
        json.put("sha256", sha256)
        json.put("direction", direction)
        return json.toString()
    }
}

/**
 * File chunk data.
 */
data class TransferChunk(
    val transferId: String,
    val sequence: Int,
    val byteOffset: Long,
    val data: ByteArray
) {
    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (other !is TransferChunk) return false
        return transferId == other.transferId && sequence == other.sequence && byteOffset == other.byteOffset && data.contentEquals(other.data)
    }

    override fun hashCode(): Int {
        var result = transferId.hashCode()
        result = 31 * result + sequence
        result = 31 * result + byteOffset.hashCode()
        result = 31 * result + data.contentHashCode()
        return result
    }
}

/**
 * Android receiver managing file staging, integrity verification, and path containment.
 */
class AndroidFileReceiver(
    private val destinationDirectory: File,
    val announce: TransferAnnounce
) {
    private val sanitizedName: String
    private val targetFile: File
    private val partFile: File
    private var randomAccessFile: RandomAccessFile? = null
    var transferredBytes: Long = 0
        private set
    var isCancelled: Boolean = false
        private set
    var isCompleted: Boolean = false
        private set

    init {
        sanitizedName = sanitizeFilename(announce.fileName)
        targetFile = File(destinationDirectory, sanitizedName)

        // Path containment check (prevent directory traversal)
        val canonicalDest = destinationDirectory.canonicalPath
        val canonicalTarget = targetFile.canonicalPath
        if (!canonicalTarget.startsWith(canonicalDest)) {
            throw SecurityException("Path traversal attempt detected: ${announce.fileName}")
        }

        if (!destinationDirectory.exists()) {
            destinationDirectory.mkdirs()
        }

        partFile = File(destinationDirectory, "$sanitizedName.part")
        if (partFile.exists()) {
            partFile.delete()
        }
        randomAccessFile = RandomAccessFile(partFile, "rw")
    }

    fun writeChunk(chunk: TransferChunk) {
        if (isCancelled) throw IllegalStateException("Transfer has been cancelled")
        if (chunk.transferId != announce.transferId) throw IllegalArgumentException("Mismatched transfer ID")

        val raf = randomAccessFile ?: throw IllegalStateException("Staging file is closed")
        raf.seek(chunk.byteOffset)
        raf.write(chunk.data)
        transferredBytes += chunk.data.size
    }

    /**
     * Finalize transfer: verify SHA-256 and atomically rename .part to target filename.
     */
    fun finish(): File {
        if (isCancelled) throw IllegalStateException("Transfer cancelled")

        randomAccessFile?.close()
        randomAccessFile = null

        val actualSha256 = calculateSha256(partFile)
        if (!actualSha256.equals(announce.sha256, ignoreCase = true)) {
            partFile.delete()
            throw IllegalStateException("Integrity check failed! Expected ${announce.sha256}, got $actualSha256")
        }

        if (targetFile.exists()) {
            targetFile.delete()
        }
        val renamed = partFile.renameTo(targetFile)
        if (!renamed) {
            throw IllegalStateException("Failed to rename temporary file to destination")
        }

        isCompleted = true
        return targetFile
    }

    fun cancel() {
        isCancelled = true
        randomAccessFile?.close()
        randomAccessFile = null
        if (partFile.exists()) {
            partFile.delete()
        }
    }

    companion object {
        fun sanitizeFilename(name: String): String {
            val trimmed = name.trim()
            if (trimmed.isEmpty() || trimmed.contains("..") || trimmed.contains("/") || trimmed.contains("\\")) {
                throw IllegalArgumentException("Invalid or unsafe filename: $name")
            }
            return trimmed
        }

        fun calculateSha256(file: File): String {
            val digest = MessageDigest.getInstance("SHA-256")
            FileInputStream(file).use { fis ->
                val buffer = ByteArray(64 * 1024)
                var read: Int
                while (fis.read(buffer).also { read = it } != -1) {
                    digest.update(buffer, 0, read)
                }
            }
            return digest.digest().joinToString("") { "%02x".format(it) }
        }
    }
}
