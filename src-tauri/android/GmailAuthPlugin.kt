package com.paisapilot.gmail

import android.app.Activity
import androidx.activity.result.ActivityResult
import androidx.activity.result.IntentSenderRequest
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import com.google.android.gms.auth.api.identity.AuthorizationRequest
import com.google.android.gms.auth.api.identity.Identity
import com.google.android.gms.auth.api.identity.RevokeAccessRequest
import com.google.android.gms.common.api.ApiException
import com.google.android.gms.common.api.Scope

private const val GMAIL_READONLY_SCOPE = "https://www.googleapis.com/auth/gmail.readonly"

@TauriPlugin
class GmailAuthPlugin(private val activity: Activity) : Plugin(activity) {
    @Command
    fun authorize(invoke: Invoke) {
        val request = AuthorizationRequest.builder()
            .setRequestedScopes(listOf(Scope(GMAIL_READONLY_SCOPE)))
            .build()

        Identity.getAuthorizationClient(activity)
            .authorize(request)
            .addOnSuccessListener { result ->
                if (result.hasResolution()) {
                    val pendingIntent = result.pendingIntent
                    if (pendingIntent == null) {
                        invoke.reject("Google requires consent but did not provide a consent request.")
                        return@addOnSuccessListener
                    }
                    val intentSenderRequest =
                        IntentSenderRequest.Builder(pendingIntent.intentSender).build()
                    startIntentSenderForResult(invoke, intentSenderRequest, "authorizationResult")
                } else {
                    resolveAuthorization(invoke, result.accessToken)
                }
            }
            .addOnFailureListener { error ->
                invoke.reject("Google authorization failed: ${error.localizedMessage ?: "unknown error"}")
            }
    }

    @ActivityCallback
    private fun authorizationResult(invoke: Invoke, result: ActivityResult) {
        if (result.resultCode != Activity.RESULT_OK) {
            invoke.reject("Google Gmail access was not granted.")
            return
        }
        val intent = result.data
        if (intent == null) {
            invoke.reject("Google returned an empty authorization response.")
            return
        }
        try {
            val authorization =
                Identity.getAuthorizationClient(activity).getAuthorizationResultFromIntent(intent)
            resolveAuthorization(invoke, authorization.accessToken)
        } catch (error: ApiException) {
            invoke.reject("Could not complete Google authorization: ${error.localizedMessage ?: "unknown error"}")
        }
    }

    @Command
    fun disconnect(invoke: Invoke) {
        val request = RevokeAccessRequest.builder()
            .setScopes(listOf(Scope(GMAIL_READONLY_SCOPE)))
            .build()
        Identity.getAuthorizationClient(activity)
            .revokeAccess(request)
            .addOnSuccessListener { invoke.resolve() }
            .addOnFailureListener { error ->
                invoke.reject("Could not revoke Gmail access: ${error.localizedMessage ?: "unknown error"}")
            }
    }

    private fun resolveAuthorization(invoke: Invoke, accessToken: String?) {
        if (accessToken.isNullOrBlank()) {
            invoke.reject("Google did not return a Gmail access token.")
            return
        }
        val result = JSObject()
        result.put("accessToken", accessToken)
        invoke.resolve(result)
    }
}
