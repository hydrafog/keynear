import { useRef } from 'react';
import { useApp } from '../../state/AppProvider';
import { Check, Palette, X, ChevronDown, ChevronRight, ChevronUp } from 'lucide-react';
import { Switch } from '../../design-system/Switch';
import { SettingRow, SettingsSection } from '../../design-system/Setting';
import { ACCENT_PRESETS, DEFAULT_ACCENT, getOnAccentColor } from '../../lib/accent';

const FOCUS_RING = 'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-kn-focus';

export function SettingsScreen() {
  const { darkMode, setDarkMode, accentColor, setAccentColor, setSubScreen, config, setConfig, showMimeEditor, setShowMimeEditor, newMimeInput, setNewMimeInput, showSnackbar, missingPermissionsCount, addIgnoredMime, removeIgnoredMime, renderStamp } = useApp();
  const colorInputRef = useRef<HTMLInputElement | null>(null);
  const isCustomAccent = !ACCENT_PRESETS.some((preset) => preset.hex.toLowerCase() === accentColor.toLowerCase());
  return (
            <div className=" space-y-6">

              <SettingsSection title="General &amp; Storage">
                <button
                  type="button"
                  onClick={() => setSubScreen('permissions')}
                  className={`flex min-h-(--kn-size-66px) w-full items-center justify-between gap-3 rounded-kn-xl px-4 py-3 text-left transition-colors duration-(--kn-duration-150) active:scale-(--kn-press-99) kn-surface-hover`}
                >
                  <div className="min-w-0 pr-2">
                    <div className="text-kn-base font-kn-semibold">System permissions</div>
                    <div className={`mt-0.5 text-kn-xs kn-text-muted`}>
                      Bluetooth, Wi-Fi, notifications, and battery access
                    </div>
                  </div>
                  <span className="flex shrink-0 items-center gap-2">
                    <span
                      className={`text-kn-sm font-kn-bold ${
                        missingPermissionsCount > 0 ? 'kn-error-text' : 'kn-text-muted'
                      }`}
                    >
                      {missingPermissionsCount > 0
                        ? `${missingPermissionsCount} missing`
                        : 'Allowed'}
                    </span>
                    <ChevronRight className={`h-5 w-5 kn-text-muted`} />
                  </span>
                </button>

                <div className="flex min-h-(--kn-size-66px) items-center justify-between gap-3 rounded-kn-xl px-4 py-3">
                  <div className="min-w-0 pr-2">
                    <div className="text-kn-base font-kn-semibold">Save folder</div>
                    <div className={`mt-0.5 text-kn-xs kn-text-muted`}>
                      Where received files are saved
                    </div>
                  </div>
                  <button
                    type="button"
                    onClick={() => {
                      const nextDir =
                        config.incomingDirectory === 'Downloads/Keynear'
                          ? 'Downloads/'
                          : 'Downloads/Keynear';
                      setConfig((c) => ({ ...c, incomingDirectory: nextDir }));
                      showSnackbar(`Save folder set to ${nextDir}.`);
                    }}
                    className={`min-h-(--kn-size-44px) shrink-0 rounded-kn-xl px-4 text-kn-xs font-kn-bold transition-colors duration-(--kn-duration-150) active:scale-(--kn-press-95) kn-surface-tonal`}
                  >
                    {config.incomingDirectory}
                  </button>
                </div>

                <SettingRow
                  title="Auto-accept from paired devices"
                  description="Save files from paired devices without asking"
                >
                  <Switch
                    key={renderStamp}
                    checked={config.autoAcceptFromPaired}
                    label="Auto-accept from paired devices"
                    onChange={(val) => setConfig((c) => ({ ...c, autoAcceptFromPaired: val }))}
                  />
                </SettingRow>
              </SettingsSection>

              <SettingsSection title="Clipboard">
                <SettingRow title="Clipboard sync" description="Keep clipboard text in sync">
                  <Switch
                    key={renderStamp}
                    checked={config.clipboardSyncEnabled}
                    label="Clipboard sync"
                    onChange={(val) => setConfig((c) => ({ ...c, clipboardSyncEnabled: val }))}
                  />
                </SettingRow>

                <SettingRow title="Sync clipboard images" description="Include images in clipboard sync">
                  <Switch
                    key={renderStamp}
                    checked={config.clipboardImagesEnabled}
                    label="Sync clipboard images"
                    onChange={(val) => setConfig((c) => ({ ...c, clipboardImagesEnabled: val }))}
                  />
                </SettingRow>

                <SettingRow title="2FA code pass-through" description="Pass verification codes to the computer">
                  <Switch
                    key={renderStamp}
                    checked={config.otpPassthroughEnabled}
                    label="2FA code pass-through"
                    onChange={(val) => setConfig((c) => ({ ...c, otpPassthroughEnabled: val }))}
                  />
                </SettingRow>

                <div>
                  <button
                    type="button"
                    aria-expanded={showMimeEditor}
                    onClick={() => setShowMimeEditor(!showMimeEditor)}
                    className={`flex min-h-(--kn-size-66px) w-full items-center justify-between gap-3 rounded-kn-xl px-4 py-3 text-left transition-colors duration-(--kn-duration-150) kn-surface-hover`}
                  >
                    <div className="min-w-0 pr-2">
                      <div className="text-kn-base font-kn-semibold">Ignored clipboard formats</div>
                      <div className={`mt-0.5 text-kn-xs kn-text-muted`}>
                        Formats that are never shared
                      </div>
                    </div>
                    <span className="flex shrink-0 items-center gap-2">
                      <span className={`text-kn-sm font-kn-bold tabular-nums kn-text-muted`}>
                        {config.ignoredMimeTypes.length}
                      </span>
                      {showMimeEditor ? (
                        <ChevronUp className={`h-5 w-5 kn-text-muted`} />
                      ) : (
                        <ChevronDown className={`h-5 w-5 kn-text-muted`} />
                      )}
                    </span>
                  </button>

                  {showMimeEditor && (
                    <div className="animate-kn-fade-in px-4 pb-4 pt-1">
                      <div className="flex flex-wrap gap-2">
                        {config.ignoredMimeTypes.map((mime) => (
                          <span
                            key={mime}
                            className={`inline-flex min-h-(--kn-size-38px) items-center gap-1 rounded-kn-xl py-1.5 pl-3.5 pr-1 text-kn-xs font-kn-semibold kn-surface-tonal`}
                          >
                            <span>{mime}</span>
                            <button
                              type="button"
                              onClick={() => removeIgnoredMime(mime)}
                              aria-label={`Remove ${mime}`}
                              className="-my-2.5 flex h-11 w-11 shrink-0 cursor-pointer items-center justify-center rounded-kn-lg transition-transform duration-(--kn-duration-press) ease-(--ease-kn-tactile) active:scale-(--kn-press-95) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-kn-focus"
                            >
                              <span className="flex h-7 w-7 items-center justify-center rounded-kn-lg kn-surface kn-text-secondary">
                                <X className="h-4 w-4" aria-hidden="true" />
                              </span>
                            </button>
                          </span>
                        ))}
                      </div>

                      <form onSubmit={addIgnoredMime} className="mt-3.5 flex gap-2.5">
                        <label htmlFor="mime-input" className="sr-only">
                          Add ignored MIME type
                        </label>
                        <input
                          id="mime-input"
                          type="text"
                          value={newMimeInput}
                          onChange={(e) => setNewMimeInput(e.target.value)}
                          placeholder="application/x-secret"
                          className={`min-h-(--kn-size-50px) min-w-0 flex-1 rounded-kn-xl px-4 text-kn-base focus-visible:outline-2 focus-visible:outline-kn-focus kn-input`}
                        />
                        <button
                          type="submit"
                          className={`min-h-(--kn-size-50px) rounded-kn-xl px-5 text-kn-sm font-kn-bold transition-all duration-(--kn-duration-150) active:scale-(--kn-press-95) kn-loading-bg`}
                        >
                          Add
                        </button>
                      </form>
                    </div>
                  )}
                </div>
              </SettingsSection>

              <SettingsSection title="Audio &amp; Notifications">
                <SettingRow title="Smart audio handoff" description="Move audio between devices">
                  <Switch
                    key={renderStamp}
                    checked={config.audioHandoffEnabled}
                    label="Smart audio handoff"
                    onChange={(val) => setConfig((c) => ({ ...c, audioHandoffEnabled: val }))}
                  />
                </SettingRow>

                <SettingRow
                  title="Notification mirroring"
                  description="Mirror selected phone notifications to desktop"
                >
                  <Switch
                    key={renderStamp}
                    checked={config.notificationMirrorEnabled}
                    label="Notification mirroring"
                    onChange={(val) => setConfig((c) => ({ ...c, notificationMirrorEnabled: val }))}
                  />
                </SettingRow>

                <SettingRow title="Completion notifications" description="Notify when files finish saving">
                  <Switch
                    key={renderStamp}
                    checked={config.notifyAutoAcceptComplete}
                    label="Completion notifications"
                    onChange={(val) => setConfig((c) => ({ ...c, notifyAutoAcceptComplete: val }))}
                  />
                </SettingRow>
              </SettingsSection>

              <SettingsSection title="Appearance">
                <SettingRow title="Dark theme" description="Switch the app color theme">
                  <Switch
                    key={renderStamp}
                    checked={darkMode}
                    label="Dark theme"
                    onChange={(val) => setDarkMode(val)}
                  />
                </SettingRow>

                <div className="rounded-kn-xl px-4 py-3.5">
                  <div className="text-kn-base font-kn-semibold">Accent color</div>
                  <div className="mt-0.5 text-kn-xs kn-text-muted">Choose the app accent color</div>
                  <div role="group" aria-label="Accent color" className="mt-3 flex items-center gap-2.5">
                    {ACCENT_PRESETS.map((preset) => {
                      const selected = accentColor.toLowerCase() === preset.hex.toLowerCase();
                      return (
                        <button
                          key={preset.id}
                          type="button"
                          onClick={() => setAccentColor(preset.hex)}
                          aria-label={`${preset.name} accent color`}
                          aria-pressed={selected}
                          className={`relative flex h-12 w-12 shrink-0 cursor-pointer items-center justify-center rounded-kn-xl transition-transform duration-(--kn-duration-press) ease-(--ease-kn-tactile) active:scale-(--kn-press-95) ${FOCUS_RING} ${
                            selected
                              ? 'ring-2 ring-[color:var(--kn-focus-ring)] ring-offset-2 ring-offset-[var(--kn-surface)]'
                              : ''
                          }`}
                          style={{ backgroundColor: preset.hex, color: getOnAccentColor(preset.hex) }}
                        >
                          {selected && <Check className="h-5 w-5" aria-hidden="true" />}
                        </button>
                      );
                    })}
                    <button
                      type="button"
                      onClick={() => colorInputRef.current?.click()}
                      aria-label="Custom accent color"
                      aria-pressed={isCustomAccent}
                      className={`relative flex h-12 w-12 shrink-0 cursor-pointer items-center justify-center rounded-kn-xl transition-transform duration-(--kn-duration-press) ease-(--ease-kn-tactile) active:scale-(--kn-press-95) ${
                        isCustomAccent ? '' : 'kn-switch-track-off'
                      } ${FOCUS_RING} ${
                        isCustomAccent
                          ? 'ring-2 ring-[color:var(--kn-focus-ring)] ring-offset-2 ring-offset-[var(--kn-surface)]'
                          : ''
                      }`}
                      style={
                        isCustomAccent
                          ? { backgroundColor: accentColor, color: getOnAccentColor(accentColor) }
                          : undefined
                      }
                    >
                      {isCustomAccent ? (
                        <Check className="h-5 w-5" aria-hidden="true" />
                      ) : (
                        <Palette className="h-5 w-5 kn-loading-text" aria-hidden="true" />
                      )}
                      <input
                        ref={colorInputRef}
                        type="color"
                        value={isCustomAccent ? accentColor : DEFAULT_ACCENT}
                        onChange={(e) => setAccentColor(e.target.value)}
                        className="sr-only"
                        tabIndex={-1}
                        aria-hidden="true"
                      />
                    </button>
                  </div>
                </div>
              </SettingsSection>
            </div>
  );
}
