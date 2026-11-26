package org.keynear.app.data

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class LocalNetworkHexTest {
    @Test
    fun endpointHexRoundTripsAndRejectsInvalidInput() {
        val bytes = ByteArray(32) { (255 - it).toByte() }
        val hex = LocalNetworkConstants.encodeHex(bytes)
        assertEquals(64, hex.length)
        assertTrue(LocalNetworkConstants.decodeHexEndpointId(hex)!!.contentEquals(bytes))
        assertTrue(LocalNetworkConstants.decodeHexEndpointId(hex.uppercase())!!.contentEquals(bytes))
        assertNull(LocalNetworkConstants.decodeHexEndpointId("xyz"))
        assertNull(LocalNetworkConstants.decodeHexEndpointId(hex.dropLast(2)))
    }
}
