package mirage.desktop.ui

import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.runComposeUiTest
import kotlin.test.Test

@OptIn(ExperimentalTestApi::class)
class SearchScreenUiTest {

    @Test
    fun `search screen shows title`() = runComposeUiTest {
        setContent {
            SearchScreen()
        }

        onNodeWithText("Mirage search").assertIsDisplayed()
    }
}
