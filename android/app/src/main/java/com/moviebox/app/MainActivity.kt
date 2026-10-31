package com.moviebox.app

import android.app.AlertDialog
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.text.InputType
import android.view.WindowManager
import android.view.inputmethod.EditorInfo
import android.widget.EditText
import android.widget.FrameLayout
import androidx.games.activity.GameActivity

/**
 * Hosts the Rust UI (libmoviebox_gui.so). The UI is drawn natively; this activity provides
 * the few things only Android can: a native text-entry dialog, the in-app video player,
 * clipboard access and back-to-home.
 */
class MainActivity : GameActivity() {

    companion object {
        init {
            System.loadLibrary("moviebox_gui")
        }

        /** Called when the user finished the text dialog. Implemented in Rust. */
        @JvmStatic
        external fun nativeOnText(id: String, text: String, submitted: Boolean)

        /** Called when the video player screen closed. Implemented in Rust. */
        @JvmStatic
        external fun nativeOnPlayerClosed()
    }

    private fun dp(value: Int): Int = (value * resources.displayMetrics.density).toInt()

    /** Show a text dialog (keyboard, paste, voice typing all work). Called from Rust. */
    fun showTextDialog(id: String, hint: String, initial: String, password: Boolean) {
        runOnUiThread {
            val input = EditText(this)
            input.setText(initial)
            input.setSelection(input.text.length)
            input.hint = hint
            input.setSingleLine(true)
            input.inputType = if (password) {
                InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_PASSWORD
            } else {
                InputType.TYPE_CLASS_TEXT
            }
            input.imeOptions = if (id == "search" || id == "ai_in" || id == "tv_url") {
                EditorInfo.IME_ACTION_SEARCH
            } else {
                EditorInfo.IME_ACTION_DONE
            }

            val container = FrameLayout(this)
            container.setPadding(dp(24), dp(8), dp(24), 0)
            container.addView(input)

            val dialog = AlertDialog.Builder(this, android.R.style.Theme_DeviceDefault_Dialog_Alert)
                .setTitle(hint)
                .setView(container)
                .setPositiveButton("OK") { _, _ -> nativeOnText(id, input.text.toString(), true) }
                .setNegativeButton("Cancel", null)
                .create()

            input.setOnEditorActionListener { _, _, _ ->
                nativeOnText(id, input.text.toString(), true)
                dialog.dismiss()
                true
            }
            dialog.window?.setSoftInputMode(WindowManager.LayoutParams.SOFT_INPUT_STATE_VISIBLE)
            dialog.show()
            input.requestFocus()
        }
    }

    /** Open the in-app player. [headers] is a flat array: name, value, name, value, ... */
    fun playVideo(url: String, headers: Array<String>, title: String) {
        runOnUiThread {
            val intent = Intent(this, PlayerActivity::class.java)
            intent.putExtra(PlayerActivity.EXTRA_URL, url)
            intent.putExtra(PlayerActivity.EXTRA_HEADERS, headers)
            intent.putExtra(PlayerActivity.EXTRA_TITLE, title)
            startActivity(intent)
        }
    }

    fun moveToBack() {
        runOnUiThread { moveTaskToBack(true) }
    }

    fun copyToClipboard(text: String) {
        runOnUiThread {
            val clipboard = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
            clipboard.setPrimaryClip(ClipData.newPlainText("MovieBox", text))
        }
    }
}
