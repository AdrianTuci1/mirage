package mirage.desktop.ui

import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.runComposeUiTest
import kotlin.test.Test
import mirage.search.InMemoryVectorStore
import mirage.search.SearchEngine

@OptIn(ExperimentalTestApi::class)
class SearchScreenUiTest {

    @Test
    fun `search screen shows search input and status bar`() = runComposeUiTest {
        setContent {
            SearchScreen(searchEngine = SearchEngine(InMemoryVectorStore()))
        }

        onNodeWithText("Search files, clipboard, vaults...").assertIsDisplayed()
        onNodeWithText("0 indexed").assertIsDisplayed()
        onNodeWithText("Start indexing").assertIsDisplayed()
        onNodeWithText("Add vault").assertIsDisplayed()
    }

}
