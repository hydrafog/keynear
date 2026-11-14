package org.keynear.app.ble

import android.Manifest
import android.annotation.SuppressLint
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothGattDescriptor
import android.bluetooth.BluetoothGattServer
import android.bluetooth.le.ScanFilter
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import android.os.ParcelUuid
import androidx.core.content.ContextCompat
import java.util.UUID

object KeynearBleConstants {
    val SERVICE_UUID: UUID = UUID.fromString("0000FE6D-0000-1000-8000-00805F9B34FB")
    val PAIRING_TX_CHAR_UUID: UUID = UUID.fromString("0000FE6E-0000-1000-8000-00805F9B34FB")
    val PAIRING_RX_CHAR_UUID: UUID = UUID.fromString("0000FE6F-0000-1000-8000-00805F9B34FB")
    val LAN_ENDPOINT_CHAR_UUID: UUID = UUID.fromString("0000FE70-0000-1000-8000-00805F9B34FB")
    val CLIENT_CONFIG_DESCRIPTOR_UUID: UUID = UUID.fromString("00002902-0000-1000-8000-00805F9B34FB")

    const val MAX_BEACON_DATA_BYTES = 24
    const val MAX_GATT_PAIRING_FRAME_BYTES = 512
    const val MAX_LAN_ENDPOINT_GATT_BYTES = 65_536
    const val ANDROID_CAPABILITY_FLAGS = 0x03u

    fun requiredPermissions(sdkInt: Int): List<String> = buildList {
        add(Manifest.permission.BLUETOOTH_SCAN)
        add(Manifest.permission.BLUETOOTH_CONNECT)
        add(Manifest.permission.BLUETOOTH_ADVERTISE)
        if (sdkInt >= 33) {
            add(Manifest.permission.POST_NOTIFICATIONS)
        }
    }

    fun missingPermissions(context: Context, sdkInt: Int): List<String> =
        requiredPermissions(sdkInt).filter { perm ->
            ContextCompat.checkSelfPermission(context, perm) != PackageManager.PERMISSION_GRANTED
        }

    fun hasScanPermission(context: Context): Boolean =
        ContextCompat.checkSelfPermission(context, Manifest.permission.BLUETOOTH_SCAN) ==
            PackageManager.PERMISSION_GRANTED

    fun hasConnectPermission(context: Context): Boolean =
        ContextCompat.checkSelfPermission(context, Manifest.permission.BLUETOOTH_CONNECT) ==
            PackageManager.PERMISSION_GRANTED

    fun hasAdvertisePermission(context: Context): Boolean =
        ContextCompat.checkSelfPermission(context, Manifest.permission.BLUETOOTH_ADVERTISE) ==
            PackageManager.PERMISSION_GRANTED

    fun buildScanFilterList(): List<ScanFilter> = listOf(
        ScanFilter.Builder()
            .setServiceUuid(ParcelUuid(SERVICE_UUID))
            .build(),
    )

    fun isValidGattFrame(bytes: ByteArray): Boolean =
        bytes.isNotEmpty() && bytes.size <= MAX_GATT_PAIRING_FRAME_BYTES
}

object KeynearBleGattCompat {
    @SuppressLint("MissingPermission")
    fun writeCharacteristicCompat(
        gatt: BluetoothGatt,
        characteristic: BluetoothGattCharacteristic,
        data: ByteArray,
    ) {
        if (android.os.Build.VERSION.SDK_INT >= 33) {
            gatt.writeCharacteristic(characteristic, data, BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT)
        } else {
            @Suppress("DEPRECATION")
            characteristic.value = data
            @Suppress("DEPRECATION")
            characteristic.writeType = BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT
            @Suppress("DEPRECATION")
            gatt.writeCharacteristic(characteristic)
        }
    }

    @SuppressLint("MissingPermission")
    fun writeDescriptorCompat(
        gatt: BluetoothGatt,
        descriptor: BluetoothGattDescriptor,
        data: ByteArray,
    ) {
        if (android.os.Build.VERSION.SDK_INT >= 33) {
            gatt.writeDescriptor(descriptor, data)
        } else {
            @Suppress("DEPRECATION")
            descriptor.value = data
            @Suppress("DEPRECATION")
            gatt.writeDescriptor(descriptor)
        }
    }

    @SuppressLint("MissingPermission")
    fun notifyCharacteristicCompat(
        server: BluetoothGattServer?,
        device: BluetoothDevice,
        characteristic: BluetoothGattCharacteristic,
        data: ByteArray,
    ) {
        if (server == null) return
        if (android.os.Build.VERSION.SDK_INT >= 33) {
            server.notifyCharacteristicChanged(device, characteristic, false, data)
        } else {
            @Suppress("DEPRECATION")
            characteristic.value = data
            @Suppress("DEPRECATION")
            server.notifyCharacteristicChanged(device, characteristic, false)
        }
    }
}
