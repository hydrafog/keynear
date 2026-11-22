package org.keynear.app

import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.keynear_core.continuityCapabilityFlags
import uniffi.keynear_core.getCoreStatus

class NativeLibrarySmokeTest {
    @Test
    fun nativeLoadRegistersCoreStatusAndContinuitySymbols() {
        System.loadLibrary("keynear_core")
        val status = getCoreStatus()
        assertTrue(status.version.isNotBlank())
        assertTrue(status.maxControlFrameBytes > 0u)
        assertTrue(status.ready)
        assertTrue(continuityCapabilityFlags() != 0u)
    }
}
