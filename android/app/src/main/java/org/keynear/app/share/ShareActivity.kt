package org.keynear.app.share

import android.content.Intent
import android.os.Bundle
import android.util.Log
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.keynear.app.MainActivity
import org.keynear.app.bridge.CoreClientProvider
import org.keynear.app.bridge.validateShareIntent
import org.keynear.app.ui.model.CoreSnapshot
import org.keynear.app.ui.model.CoreState
import org.keynear.app.ui.model.ShareValidationResult
import org.keynear.designsystem.KeynearTheme

private val SheetCornerTop = 28.dp
private val SheetElevation = 16.dp
private val HandleWidth = 36.dp
private val HandleHeight = 4.dp
private val CloseBadgeSize = 36.dp
private val CardCornerRadius = 20.dp
private val DeviceGridMaxHeight = 320.dp
private val DeviceTileCornerRadius = 18.dp
private val DeviceTileMinHeight = 96.dp
private val DeviceIconContainerSize = 44.dp
private val DeviceIconContainerRadius = 14.dp
private val PillCorner = 999.dp
private val SheetGap = 14.dp
private val SectionGap = 10.dp
private val HeaderTextSize = 19.sp
private val EmptyHeaderSize = 15.sp
private val SmallHeaderSize = 13.sp
private val CaptionLetterSpacing = 0.8.sp
private val MicroLabelSize = 11.sp

class ShareActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val incoming = intent
        val action = incoming?.action
        val streamUris = IntentResolver.extractStreamUris(incoming, action)
        val sharedText = incoming?.getStringExtra(Intent.EXTRA_TEXT)
        val preselectedDeviceId = IntentResolver.extractPreselectedDeviceId(incoming)
        val validation = validateShareIntent(action, streamUris, sharedText)

        when (validation) {
            is ShareValidationResult.Valid -> {
                Log.i(
                    TAG,
                    "share_intent_validated items=${validation.itemCount} has_text=${validation.hasText}",
                )
            }
            is ShareValidationResult.Invalid -> {
                Log.w(TAG, "share_intent_rejected reason=${validation.reason}")
            }
        }

        val client = CoreClientProvider.init(applicationContext)
        setContent {
            ShareActivityContent(
                client = client,
                validation = validation,
                streamUris = streamUris,
                sharedText = sharedText,
                preselectedDeviceId = preselectedDeviceId,
                onOpenMain = {
                    startActivity(Intent(this, MainActivity::class.java))
                    finish()
                },
                onClose = { finish() },
            )
        }
    }

    companion object {
        private const val TAG = "ShareActivity"
    }
}
