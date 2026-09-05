import QtQuick
import qs.Commons
import qs.Ui

// Settings for the launcher. `host` is the Omarchycast root, which owns the config
// object and the socket; this pane only edits and saves it.
Flickable {
  id: pane

  property var host

  readonly property color fg: host.foreground
  readonly property color accentColor: host.selectedBackground
  readonly property string fontFamily: host.fontFamily
  function fs(px) { return host.fs(px) }
  property bool capturing: false

  focus: true
  contentWidth: width
  contentHeight: layout.implicitHeight + Style.space(24)
  clip: true
  boundsBehavior: Flickable.StopAtBounds

  function commitTop(key, value) {
    var next = JSON.parse(JSON.stringify(host.config))
    next[key] = value
    host.config = next
    host.saveConfig()
  }

  function commit(section, key, value) {
    // QML won't notice a mutation inside the object, so the whole config is
    // rebuilt to force the bindings that read it to re-evaluate.
    var next = JSON.parse(JSON.stringify(host.config))
    next[section][key] = value
    host.config = next
    host.saveConfig()
  }

  function commitPrefixes(prefixes) {
    var next = JSON.parse(JSON.stringify(host.config))
    next.providers.websearchPrefixes = prefixes
    host.config = next
    host.saveConfig()
  }

  // Qt delivers modifiers separately from the key, so a readable binding string
  // has to be assembled by hand.
  function keyName(code) {
    if (code >= Qt.Key_A && code <= Qt.Key_Z) return String.fromCharCode(65 + (code - Qt.Key_A))
    if (code >= Qt.Key_0 && code <= Qt.Key_9) return String.fromCharCode(48 + (code - Qt.Key_0))
    if (code >= Qt.Key_F1 && code <= Qt.Key_F12) return "F" + (1 + (code - Qt.Key_F1))
    switch (code) {
      case Qt.Key_Space: return "SPACE"
      case Qt.Key_Return:
      case Qt.Key_Enter: return "RETURN"
      case Qt.Key_Tab: return "TAB"
      case Qt.Key_Backspace: return "BACKSPACE"
      case Qt.Key_Period: return "PERIOD"
      case Qt.Key_Comma: return "COMMA"
      case Qt.Key_Slash: return "SLASH"
      default: return ""
    }
  }

  function describe(event) {
    var parts = []
    if (event.modifiers & Qt.MetaModifier) parts.push("SUPER")
    if (event.modifiers & Qt.ControlModifier) parts.push("CTRL")
    if (event.modifiers & Qt.AltModifier) parts.push("ALT")
    if (event.modifiers & Qt.ShiftModifier) parts.push("SHIFT")
    var name = pane.keyName(event.key)
    if (name === "") return ""
    parts.push(name)
    return parts.join(" + ")
  }

  // ------------------------------------------------------------- small pieces

  component SectionTitle: Text {
    color: pane.fg
    opacity: 0.5
    font.family: pane.fontFamily
    font.pixelSize: pane.fs(Style.font.caption)
    font.letterSpacing: 1
    topPadding: Style.space(10)
  }

  component Toggle: Item {
    id: toggle
    property string label
    property string hint
    property bool checked
    signal toggled(bool value)

    width: parent ? parent.width : 0
    height: Math.max(Style.space(34), text.implicitHeight + Style.space(10))

    MouseArea {
      anchors.fill: parent
      onClicked: toggle.toggled(!toggle.checked)
    }

    Column {
      id: text
      anchors.left: parent.left
      anchors.right: box.left
      anchors.rightMargin: Style.space(12)
      anchors.verticalCenter: parent.verticalCenter
      spacing: Style.space(1)

      Text {
        width: parent.width
        text: toggle.label
        color: pane.fg
        elide: Text.ElideRight
        font.family: pane.fontFamily
        font.pixelSize: pane.fs(Style.font.body)
      }
      Text {
        width: parent.width
        visible: !!toggle.hint
        text: toggle.hint || ""
        color: pane.fg
        opacity: 0.5
        elide: Text.ElideRight
        font.family: pane.fontFamily
        font.pixelSize: pane.fs(Style.font.caption)
      }
    }

    Rectangle {
      id: box
      anchors.right: parent.right
      anchors.verticalCenter: parent.verticalCenter
      width: Style.space(38)
      height: Style.space(21)
      radius: height / 2
      color: toggle.checked ? pane.accentColor : "transparent"
      border.color: pane.fg
      border.width: 1
      opacity: toggle.checked ? 1.0 : 0.45

      Rectangle {
        width: Style.space(15)
        height: width
        radius: width / 2
        color: pane.fg
        anchors.verticalCenter: parent.verticalCenter
        x: toggle.checked ? parent.width - width - Style.space(3) : Style.space(3)
      }
    }
  }

  component StepButton: Rectangle {
    id: stepButton
    property string glyph
    signal activated

    width: Style.space(24)
    height: Style.space(24)
    radius: Style.space(6)
    color: "transparent"
    border.color: pane.fg
    border.width: 1
    opacity: 0.55

    Text {
      anchors.centerIn: parent
      text: stepButton.glyph
      color: pane.fg
      font.family: pane.fontFamily
      font.pixelSize: pane.fs(Style.font.caption)
    }
    MouseArea { anchors.fill: parent; onClicked: stepButton.activated() }
  }

  component PathField: Item {
    id: field
    property string label
    property string value
    property string placeholder
    signal updated(string value)

    width: parent ? parent.width : 0
    height: Style.space(58)

    Text {
      anchors.left: parent.left
      anchors.top: parent.top
      text: field.label
      color: pane.fg
      font.family: pane.fontFamily
      font.pixelSize: pane.fs(Style.font.body)
    }

    Rectangle {
      anchors.left: parent.left
      anchors.right: parent.right
      anchors.bottom: parent.bottom
      anchors.bottom