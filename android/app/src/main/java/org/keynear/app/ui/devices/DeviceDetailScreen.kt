package org.keynear.app.ui.devices

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.keynear.app.ui.model.DeviceRoleUi
import org.keynear.app.ui.model.PairedDeviceUi
import org.keynear.designsystem.ActionButton
import org.keynear.designsystem.KeynearSwitch
import org.keynear.designsystem.RecessedInput
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone
import org.keynear.designsystem.IconLaptop
import org.keynear.designsystem.IconRefreshCw
import org.keynear.designsystem.IconSmartphone
import org.keynear.designsystem.IconTrash2
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.designsystem.KeynearLayout
import org.keynear.designsystem.KeynearRadii
import org.keynear.designsystem.KeynearShapes
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.KeynearTypeSizes
import org.keynear.designsystem.LocalKeynearPalette
private val ActionIconSize = 20.dp
private val ContentGap = 10.dp
private val ControlMinHeight = 52.dp
private val DetailRowMinHeight = 54.dp
private val DeviceNameSize = 18.sp
private val HeaderIconSize = 56.dp
private val RowIconSize = 14.dp
private val SectionIconSize = 28.dp
private val TextSubtitleGap = 2.dp
private val ValueSize = 22.sp


@Composable
fun DeviceDetailScreen(
    device: PairedDeviceUi?,
    confirmingUnpair: Boolean,
    actions: KeynearActions,
    modifier: Modifier = Modifier,
) {
    val palette = LocalKeynearPalette.current
    if (device == null) {
        Box(
            modifier = modifier
                .fillMaxSize()
                .background(palette.canvas),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = "Device no longer paired.",
                color = palette.textMuted,
                fontSize = KeynearTypeSizes.Description,
            )
        }
        return
    }

    var renameDraft by remember(device.deviceId, device.displayName) {
        mutableStateOf(device.displayName)
    }

    Column(
        modifier = modifier
            .fillMaxSize()
            .background(palette.canvas)
            .verticalScroll(rememberScrollState())
            .padding(
                horizontal = KeynearLayout.ScreenPaddingHorizontal,
                vertical = KeynearLayout.ScreenPaddingVertical,
            ),
        verticalArrangement = Arrangement.spacedBy(KeynearLayout.CompactSectionGap),
    ) {
        DeviceHeaderCard(device = device)

        DeviceSettingsCard(
            device = device,
            renameDraft = renameDraft,
            onRenameDraftChange = { renameDraft = it },
            actions = actions,
        )

        DeviceActionsColumn(
            device = device,
            confirmingUnpair = confirmingUnpair,
            actions = actions,
        )

        Spacer(modifier = Modifier.height(KeynearSpacing.Sm))
    }
}
