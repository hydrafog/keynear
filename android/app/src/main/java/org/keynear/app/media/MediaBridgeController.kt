package org.keynear.app.media

import android.content.Context
import android.media.MediaMetadata
import android.media.session.MediaController
import android.media.session.MediaSession
import android.media.session.PlaybackState

object MediaBridgeController {
    sealed class Command {
        object Play : Command()
        object Pause : Command()
        object Next : Command()
        object Previous : Command()
    }

    const val LABEL_PLAY: String = "Play"
    const val LABEL_PAUSE: String = "Pause"
    const val LABEL_NEXT: String = "Next"
    const val LABEL_PREVIOUS: String = "Previous"

    fun commandFromLabel(label: String): Command? {
        return when (label) {
            LABEL_PLAY -> Command.Play
            LABEL_PAUSE -> Command.Pause
            LABEL_NEXT -> Command.Next
            LABEL_PREVIOUS -> Command.Previous
            else -> null
        }
    }

    fun methodFor(command: Command): String {
        return when (command) {
            Command.Play -> "Play"
            Command.Pause -> "Pause"
            Command.Next -> "Next"
            Command.Previous -> "Previous"
        }
    }

    fun createSession(context: Context, tag: String): MediaSession {
        val session = MediaSession(context, tag)
        session.isActive = true
        return session
    }

    fun publish(session: MediaSession, title: String, artist: String, playing: Boolean) {
        val metadata = MediaMetadata.Builder()
            .putString(MediaMetadata.METADATA_KEY_TITLE, title)
            .putString(MediaMetadata.METADATA_KEY_ARTIST, artist)
            .build()
        val state = if (playing) PlaybackState.STATE_PLAYING else PlaybackState.STATE_PAUSED
        val playbackState = PlaybackState.Builder()
            .setState(state, 0L, 1.0f)
            .build()
        session.setMetadata(metadata)
        session.setPlaybackState(playbackState)
    }

    fun controllerFor(session: MediaSession): MediaController? {
        return session.controller
    }
}
