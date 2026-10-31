package com.moviebox.app

import android.os.Bundle
import android.view.WindowManager
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.PlaybackException
import androidx.media3.common.Player
import androidx.media3.common.util.UnstableApi
import androidx.media3.datasource.DefaultHttpDataSource
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.exoplayer.source.DefaultMediaSourceFactory
import androidx.media3.ui.PlayerView

/** Full-screen video player (Media3 / ExoPlayer) that supports custom HTTP headers. */
@androidx.annotation.OptIn(UnstableApi::class)
class PlayerActivity : AppCompatActivity() {

    companion object {
        const val EXTRA_URL = "url"
        const val EXTRA_HEADERS = "headers"
        const val EXTRA_TITLE = "title"
        private const val DEFAULT_USER_AGENT =
            "Mozilla/5.0 (Linux; Android 13) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Mobile Safari/537.36"
    }

    private var player: ExoPlayer? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        WindowCompat.setDecorFitsSystemWindows(window, false)

        val url = intent.getStringExtra(EXTRA_URL)
        if (url.isNullOrEmpty()) {
            finish()
            return
        }
        val title = intent.getStringExtra(EXTRA_TITLE) ?: ""
        val flat = intent.getStringArrayExtra(EXTRA_HEADERS) ?: emptyArray()
        val headers = HashMap<String, String>()
        var i = 0
        while (i + 1 < flat.size) {
            headers[flat[i]] = flat[i + 1]
            i += 2
        }

        val http = DefaultHttpDataSource.Factory()
            .setAllowCrossProtocolRedirects(true)
            .setUserAgent(DEFAULT_USER_AGENT)
            .setDefaultRequestProperties(headers)
        val exo = ExoPlayer.Builder(this)
            .setMediaSourceFactory(DefaultMediaSourceFactory(this).setDataSourceFactory(http))
            .build()
        player = exo

        val view = PlayerView(this)
        view.player = exo
        view.keepScreenOn = true
        setContentView(view)
        hideSystemBars()

        exo.setMediaItem(
            MediaItem.Builder()
                .setUri(url)
                .setMediaMetadata(MediaMetadata.Builder().setTitle(title).build())
                .build()
        )
        exo.addListener(object : Player.Listener {
            override fun onPlayerError(error: PlaybackException) {
                Toast.makeText(
                    this@PlayerActivity,
                    "Could not play this stream: " + (error.message ?: "unknown error"),
                    Toast.LENGTH_LONG
                ).show()
                finish()
            }

            override fun onPlaybackStateChanged(state: Int) {
                if (state == Player.STATE_ENDED) {
                    finish()
                }
            }
        })
        exo.prepare()
        exo.playWhenReady = true
    }

    private fun hideSystemBars() {
        val controller = WindowInsetsControllerCompat(window, window.decorView)
        controller.hide(WindowInsetsCompat.Type.systemBars())
        controller.systemBarsBehavior = WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
    }

    override fun onStop() {
        super.onStop()
        player?.pause()
    }

    override fun onDestroy() {
        player?.release()
        player = null
        super.onDestroy()
        if (isFinishing) {
            MainActivity.nativeOnPlayerClosed()
        }
    }
}
