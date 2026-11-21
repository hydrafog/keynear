package org.keynear.app.ui.settings

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
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
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.keynear.app.ui.model.DaemonConfigUi
import org.keynear.app.ui.model.PermissionsChecklistState
import org.keynear.designsystem.ActionButton
import org.keynear.designsystem.KeynearSwitch
import org.keynear.designsystem.RecessedInput
import org.keynear.designsystem.SettingRow
import org.keynear.designsystem.SettingsSection
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone
import org.keynear.designsystem.IconChevronDown
import org.keynear.designsystem.IconChevronRight
import org.keynear.designsystem.IconChevronUp
import org.keynear.designsystem.IconClose
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.designsystem.KeynearLayout
import org.keynear.designsystem.KeynearRadii
import org.keynear.designsystem.KeynearShapes
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.KeynearTypeSizes
import org.keynear.designsystem.LocalKeynearPalette

private val ActionMinHeight = 44.dp
private val ChipMinWidth = 50.dp
private val CompactGap = 6.dp
internal val ContentGap = 10.dp
private val ControlIconSize = 20.dp
private val FieldHeight = 38.dp
internal val RowIconSize = 14.dp
internal val ValueTextSize = 19.sp


@OptIn(ExperimentalLayoutApi::class)
@Composable
fun SettingsScreen(
    config: DaemonConfigUi,
    permissions: PermissionsChecklistState,
    actions: KeynearActions,
    modifier: Modifier = Modifier,
) {
    val palette = LocalKeynearPalette.current
    Column(
        modifier = modifier
            .fillMaxSize()
            .background(palette.canvas)
            .verticalScroll(rememberScrollState())
            .padding(
                horizontal = KeynearLayout.ScreenPaddingHorizontal,
                vertical = KeynearLayout.ScreenPaddingVertical,
            ),
        verticalArrangement = Arrangement.spacedBy(KeynearLayout.SectionGap),
    ) {
        GeneralStorageSection(config = config, permissions = permissions, actions = actions)
        ClipboardSettingsSection(config = config, actions = actions)
        AudioNotificationSection(config = config, actions = actions)
        AppearanceSection(config = config, actions = actions)

        Spacer(modifier = Modifier.height(KeynearSpacing.Sm))
    }
}
