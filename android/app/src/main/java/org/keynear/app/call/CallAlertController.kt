package org.keynear.app.call

import android.Manifest
import android.annotation.SuppressLint
import android.content.Context
import android.media.AudioManager
import android.content.pm.PackageManager
import android.telecom.Call
import android.telecom.CallScreeningService
import android.telecom.TelecomManager
import org.keynear.app.notifications.NotificationMirror

object CallAlertController {
    sealed class Action {
        object Answer : Action()
        object Decline : Action()
        object Mute : Action()
    }

    fun actionFromLabel(label: String): Action? {
        return when (label) {
            NotificationMirror.LABEL_ANSWER -> Action.Answer
            NotificationMirror.LABEL_DECLINE -> Action.Decline
            NotificationMirror.LABEL_MUTE -> Action.Mute
            else -> null
        }
    }

    fun apply(context: Context, action: Action): Boolean {
        val manager = context.getSystemService(TelecomManager::class.java) ?: return false
        return when (action) {
            Action.Answer -> answer(manager, context)
            Action.Decline -> decline(manager, context)
            Action.Mute -> mute(context, manager)
        }
    }

    private fun canControlCalls(context: Context): Boolean {
        return context.checkSelfPermission(Manifest.permission.ANSWER_PHONE_CALLS) ==
            PackageManager.PERMISSION_GRANTED
    }

    @SuppressLint("MissingPermission")
    private fun answer(manager: TelecomManager, context: Context): Boolean {
        if (!canControlCalls(context) || !manager.isInCall) return false
        manager.acceptRingingCall()
        return true
    }

    @SuppressLint("MissingPermission")
    private fun decline(manager: TelecomManager, context: Context): Boolean {
        if (!canControlCalls(context) || !manager.isInCall) return false
        manager.endCall()
        return true
    }

    @SuppressLint("MissingPermission")
    private fun mute(context: Context, manager: TelecomManager): Boolean {
        if (!manager.isInCall) return false
        val audio = context.getSystemService(AudioManager::class.java) ?: return false
        audio.isMicrophoneMute = true
        return true
    }
}

class KeynearCallScreeningService : CallScreeningService() {
    override fun onScreenCall(callDetails: Call.Details) {
        respondToCall(
            callDetails,
            CallResponse.Builder().setDisallowCall(true).setRejectCall(true).build(),
        )
    }
}
