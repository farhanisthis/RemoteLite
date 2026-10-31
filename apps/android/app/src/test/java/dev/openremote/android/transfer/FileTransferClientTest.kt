package dev.openremote.android.transfer

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import java.io.File
import java.security.MessageDigest

class FileTransferClientTest {

    @get:Rule
    val tempFolder = TemporaryFolder()

    @Test
    fun testReceiveFileSuccess() {
        val destDir = tempFolder.newFolder("downloads")
        val content = "OpenRemote Android File Receiver Test Payload".toByteArray(Charsets.UTF_8)

        val digest = MessageDigest.getInstance("SHA-256")
        val expectedSha256 = digest.digest(content).joinToString("") { "%02x".format(it) }

        val announce = TransferAnnounce(
            transferId = "transfer-1234",
            fileName = "notes.txt",
            fileSize = content.size.toLong(),
            sha256 = expectedSha256,
            direction = "host_to_client"
        )

        val receiver = AndroidFileReceiver(destDir, announce)

        // Split into 2 chunks
        val chunk1 = TransferChunk("transfer-1234", 0, 0, content.copyOfRange(0, 15))
        val chunk2 = TransferChunk("transfer-1234", 1, 15, content.copyOfRange(15, content.size))

        receiver.writeChunk(chunk1)
        receiver.writeChunk(chunk2)

        val finalFile = receiver.finish()
        assertTrue(finalFile.exists())
        assertEquals("notes.txt", finalFile.name)
        assertEquals(String(content), finalFile.readText())

        // Ensure staging file is cleaned up
        val partFile = File(destDir, "notes.txt.part")
        assertFalse(partFile.exists())
    }

    @Test
    fun testIntegrityCheckFailurePurgesPartFile() {
        val destDir = tempFolder.newFolder("downloads_corrupt")
        val content = "Correct data".toByteArray(Charsets.UTF_8)

        val announce = TransferAnnounce(
            transferId = "transfer-bad",
            fileName = "corrupt.bin",
            fileSize = content.size.toLong(),
            sha256 = "0000000000000000000000000000000000000000000000000000000000000000",
            direction = "host_to_client"
        )

        val receiver = AndroidFileReceiver(destDir, announce)
        receiver.writeChunk(TransferChunk("transfer-bad", 0, 0, content))

        var failed = false
        try {
            receiver.finish()
        } catch (e: IllegalStateException) {
            failed = true
        }

        assertTrue(failed)
        assertFalse(File(destDir, "corrupt.bin").exists())
        assertFalse(File(destDir, "corrupt.bin.part").exists())
    }

    @Test
    fun testPathTraversalRejected() {
        val destDir = tempFolder.newFolder("downloads_secure")

        var rejected = false
        try {
            val announce = TransferAnnounce(
                transferId = "transfer-evil",
                fileName = "../../../etc/passwd",
                fileSize = 100,
                sha256 = "dummy",
                direction = "host_to_client"
            )
            AndroidFileReceiver(destDir, announce)
        } catch (e: IllegalArgumentException) {
            rejected = true
        }

        assertTrue(rejected)
    }

    @Test
    fun testCancellationPurgesStagingFile() {
        val destDir = tempFolder.newFolder("downloads_cancel")
        val content = "Partial payload".toByteArray()

        val announce = TransferAnnounce(
            transferId = "transfer-cancel",
            fileName = "cancel_me.dat",
            fileSize = 1000,
            sha256 = "dummy",
            direction = "host_to_client"
        )

        val receiver = AndroidFileReceiver(destDir, announce)
        receiver.writeChunk(TransferChunk("transfer-cancel", 0, 0, content))
        assertTrue(File(destDir, "cancel_me.dat.part").exists())

        receiver.cancel()
        assertFalse(File(destDir, "cancel_me.dat.part").exists())
        assertFalse(File(destDir, "cancel_me.dat").exists())
    }
}
