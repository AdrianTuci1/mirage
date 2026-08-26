package mirage.desktop

import androidx.compose.ui.window.Window
import androidx.compose.ui.window.application
import mirage.desktop.ui.SearchScreen

fun main() = application {
    Window(
        onCloseRequest = ::exitApplication,
        title = "Mirage"
    ) {
        SearchScreen()
    }
}
