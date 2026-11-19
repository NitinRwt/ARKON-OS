import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import SddmComponents 2.0

Rectangle {
    id: root
    width: 1920
    height: 1080
    color: "#000005"

    // Background
    Image {
        anchors.fill: parent
        source: "background.svg"
        fillMode: Image.PreserveAspectCrop
    }

    // Blur overlay
    Rectangle {
        anchors.fill: parent
        color: "transparent"
    }

    // Center login card
    Rectangle {
        id: card
        anchors.centerIn: parent
        width: 380
        height: 460
        radius: 20
        color: Qt.rgba(0.04, 0.06, 0.14, 0.85)
        border.color: Qt.rgba(0.18, 0.44, 1.0, 0.3)
        border.width: 1

        // Subtle glow behind card
        layer.enabled: true

        ColumnLayout {
            anchors.centerIn: parent
            spacing: 20
            width: parent.width - 60

            // ARKON wordmark
            Text {
                Layout.alignment: Qt.AlignHCenter
                text: "ARKON"
                font.family: "SF Pro Display, Helvetica Neue, Arial"
                font.pixelSize: 42
                font.weight: Font.Bold
                font.letterSpacing: 10
                color: "#4da6ff"
                style: Text.Outline
                styleColor: Qt.rgba(0.1, 0.4, 1.0, 0.3)
            }

            // User avatar placeholder
            Rectangle {
                Layout.alignment: Qt.AlignHCenter
                width: 80
                height: 80
                radius: 40
                color: Qt.rgba(0.18, 0.44, 1.0, 0.2)
                border.color: Qt.rgba(0.18, 0.44, 1.0, 0.5)
                border.width: 2

                Text {
                    anchors.centerIn: parent
                    text: userModel.currentUser.length > 0
                          ? userModel.currentUser[0].toUpperCase()
                          : "A"
                    font.pixelSize: 36
                    font.weight: Font.Medium
                    color: "#4da6ff"
                }
            }

            // Username display
            Text {
                Layout.alignment: Qt.AlignHCenter
                text: userModel.currentUser || "arkon"
                font.pixelSize: 16
                font.weight: Font.Medium
                color: "#c8d8f0"
                font.letterSpacing: 1
            }

            // Password field
            TextField {
                id: passwordField
                Layout.fillWidth: true
                placeholderText: "Password"
                echoMode: TextInput.Password
                font.pixelSize: 15
                color: "#e0eaff"
                placeholderTextColor: "#4a6080"
                leftPadding: 16
                rightPadding: 16
                height: 44
                background: Rectangle {
                    radius: 10
                    color: Qt.rgba(0.08, 0.12, 0.25, 0.9)
                    border.color: passwordField.activeFocus
                              ? Qt.rgba(0.18, 0.44, 1.0, 0.8)
                              : Qt.rgba(0.18, 0.44, 1.0, 0.25)
                    border.width: passwordField.activeFocus ? 1.5 : 1
                }
                Keys.onReturnPressed: sddm.login(userModel.currentUser, passwordField.text, sessionIndex)
            }

            // Login button
            Rectangle {
                Layout.fillWidth: true
                height: 44
                radius: 10
                color: loginArea.containsMouse
                       ? Qt.rgba(0.22, 0.52, 1.0, 0.95)
                       : Qt.rgba(0.18, 0.44, 1.0, 0.85)

                Behavior on color { ColorAnimation { duration: 150 } }

                Text {
                    anchors.centerIn: parent
                    text: "Sign In"
                    font.pixelSize: 15
                    font.weight: Font.SemiBold
                    color: "white"
                    font.letterSpacing: 0.5
                }

                MouseArea {
                    id: loginArea
                    anchors.fill: parent
                    hoverEnabled: true
                    onClicked: sddm.login(userModel.currentUser, passwordField.text, sessionIndex)
                }
            }

            // Message area (wrong password, etc.)
            Text {
                id: message
                Layout.alignment: Qt.AlignHCenter
                text: ""
                font.pixelSize: 13
                color: "#ff6b6b"
                visible: text.length > 0
            }
        }
    }

    // Bottom: session selector + time
    Text {
        anchors.bottom: parent.bottom
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottomMargin: 32
        text: Qt.formatDateTime(new Date(), "dddd, MMMM d  ·  hh:mm AP")
        font.pixelSize: 13
        color: Qt.rgba(1, 1, 1, 0.35)
        font.letterSpacing: 0.5
    }

    Connections {
        target: sddm
        function onLoginFailed() {
            message.text = "Incorrect password"
            passwordField.text = ""
            passwordField.forceActiveFocus()
        }
    }

    Component.onCompleted: passwordField.forceActiveFocus()
}
