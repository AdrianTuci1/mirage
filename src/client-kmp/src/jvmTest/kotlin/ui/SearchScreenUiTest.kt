package mirage.desktop.ui

import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performTextInput
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.WindowPosition
import androidx.compose.ui.window.WindowState
import kotlin.test.Test
import mirage.search.SearchResult

/**
 * Contract tests for the spotlight window, checked against the
 * "Mirage · Spotlight" boards in Penpot.
 *
 * They pin what the 2026-08 review changed: the module row and the indexing bar
 * left the window, an empty result list withdraws instead of leaving a divider
 * and a gap behind, and the only place to start a pass is Settings.
 */
@OptIn(ExperimentalTestApi::class)
class SearchScreenUiTest {

    private fun testWindowState() = WindowState(
        width = 720.dp,
        height = 480.dp,
        position = WindowPosition(0.dp, 0.dp)
    )

    private val report = SearchResult(
        id = "annual-report",
        relativePath = "~/Dropbox/Finance/Annual Report 2025.pdf",
        sourceType = "dropbox",
        score = 0.91,
        openUrl = "https://dropbox.example/annual-report-2025"
    )

    @Test
    fun idleSpotlightShowsOnlyInputAndFooter() = runComposeUiTest {
        setContent {
            SearchScreen(
                windowState = testWindowState(),
                search = { emptyList() },
                onOpenResult = {}
            )
        }

        onNodeWithTag(SEARCH_INPUT_TAG).assertIsDisplayed()
        onNodeWithText("settings").assertIsDisplayed()
        onNodeWithText("clipboard").assertIsDisplayed()

        // Dropped from this window in the review: the module row, the indexing
        // bar, the start-indexing chip and the add-server chip.
        onNodeWithText("Start indexing").assertDoesNotExist()
        onNodeWithText("Add server").assertDoesNotExist()
        onNodeWithText("Modules:").assertDoesNotExist()
        onNodeWithText("indexed").assertDoesNotExist()
    }

    @Test
    fun aQueryListsResultsWithNameAndPath() = runComposeUiTest {
        setContent {
            SearchScreen(
                windowState = testWindowState(),
                search = { listOf(report) },
                onOpenResult = {}
            )
        }

        onNodeWithTag(SEARCH_INPUT_TAG).performTextInput("annual report")
        waitForIdle()

        onNodeWithText("Annual Report 2025.pdf").assertIsDisplayed()
        onNodeWithText("~/Dropbox/Finance/Annual Report 2025.pdf").assertIsDisplayed()
        // The selected row has an open URL, so the footer offers the shortcut.
        onNodeWithText("download").assertIsDisplayed()
    }

    @Test
    fun aQueryWithoutHitsKeepsTheListWithdrawn() = runComposeUiTest {
        setContent {
            SearchScreen(
                windowState = testWindowState(),
                search = { emptyList() },
                onOpenResult = {}
            )
        }

        onNodeWithTag(SEARCH_INPUT_TAG).performTextInput("annual report")
        waitForIdle()

        onNodeWithText("~/Dropbox/Finance/Annual Report 2025.pdf").assertDoesNotExist()
        // Only the no-hits notice is left, so the window still hugs its content.
        onNodeWithText("No results for \"annual report\"").assertIsDisplayed()
    }
}
