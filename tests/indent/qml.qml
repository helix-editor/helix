import QtQuick 2.0

Rectangle {
    width: 100
    height: 100

    Text {
        text: "hi"
        anchors {
            top: parent.top
            left: parent.left
        }
    }

    Column {
        spacing: 4
    }
}
