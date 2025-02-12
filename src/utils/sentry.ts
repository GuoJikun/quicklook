import * as Sentry from '@sentry/vue'
import type { App } from 'vue'
import type { Router } from 'vue-router'
import pkg from '../../package.json'

export interface SentryConfig {
    app: App
    router: Router
}

export default (conf: SentryConfig) => {
    Sentry.init({
        app: conf.app,
        dsn: 'https://8fb284d3b45b3f203d904cabf2460287@o4507727722905600.ingest.us.sentry.io/4508803797286912',
        integrations: [
            Sentry.browserTracingIntegration({ router: conf.router }),
            Sentry.replayIntegration(),
            Sentry.vueIntegration({ tracingOptions: { trackComponents: true } }),
        ],
        // Tracing
        tracesSampleRate: 1.0, //  Capture 100% of the transactions
        // Set 'tracePropagationTargets' to control for which URLs distributed tracing should be enabled
        // tracePropagationTargets: ['localhost'],
        // Session Replay
        replaysSessionSampleRate: 0.1, // This sets the sample rate at 10%. You may want to change it to 100% while in development and then sample at a lower rate in production.
        replaysOnErrorSampleRate: 1.0, // If you're not already sampling the entire session, change the sample rate to 100% when sampling sessions where errors occur.
        release: `quicklook-vue@${pkg.version}`,
    })
}
