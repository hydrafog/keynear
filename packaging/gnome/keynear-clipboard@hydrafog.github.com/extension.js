import Gio from 'gi://Gio';
import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';

const BUS_NAME = 'org.keynear.ClipboardBridge';
const OBJECT_PATH = '/org/keynear/ClipboardBridge';

const INTERFACE_XML = `<node>
  <interface name="org.keynear.ClipboardBridge">
    <method name="Sync">
      <arg name="text" type="s" direction="in" />
    </method>
    <method name="Status">
      <arg name="status" type="s" direction="out" />
    </method>
  </interface>
</node>`;

export default class KeynearClipboardExtension extends Extension {
  enable() {
    this._text = '';
    this._connection = Gio.bus_get_sync(Gio.BusType.SESSION, null);
    this._ownedName = Gio.bus_own_name_on_connection(
      this._connection,
      BUS_NAME,
      Gio.BusNameOwnerFlags.NONE,
      null,
      null,
    );
    const info = Gio.DBusInterfaceInfo.new_for_xml(INTERFACE_XML);
    this._registration = this._connection.register_object(
      OBJECT_PATH,
      info,
      {
        Sync: ([text]) => {
          this._text = String(text);
        },
        Status: () => [this._text === '' ? 'empty' : 'ready'],
      },
      null,
      null,
    );
  }

  disable() {
    if (this._registration !== null) {
      this._connection.unregister_object(this._registration);
    }
    if (this._ownedName !== 0) {
      Gio.bus_unown_name(this._ownedName);
    }
    this._registration = null;
    this._ownedName = 0;
    this._connection = null;
    this._text = '';
  }
}
