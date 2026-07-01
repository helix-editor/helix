import QtQuick

Item {
    id: root

    property int count: 0

    function compute(a, b) {
        var result = {
            sum: a + b,
            diff: a - b
        }
        var { sum, diff } = result
        return sum + diff
    }

    Connections {
        target: Hyprland

        function onRawEvent() {
            root.count += 1
        }
    }
}
