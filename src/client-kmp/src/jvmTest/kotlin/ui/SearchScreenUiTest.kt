package mirage.desktop.ui

import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.runComposeUiTest
import kotlin.test.Test
import mirage.search.SearchResult

@OptIn(ExperimentalTestApi::class)
class SearchScreenUiTest {

    @Test
    fun `search screen shows search input and status bar`() = runComposeUiTest {
        setContent {
            SearchScreen(
                search = { emptyList() },
                onOpenResult = {},
                indexedCount = 0
            )
        }

        onNodeWithText("Search everything").assertIsDisplayed()
        onNodeWithText("0 indexed").assertIsDisplayed()
        onNodeWithText("Start indexing").assertIsDisplayed()
        onNodeWithText("Sync").assertIsDisplayed()
        onNodeWithText("Add server").assertIsDisplayed()
    }

}
