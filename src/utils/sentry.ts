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
            Sentry.vueIntegration({ tracingOptions: { trackComponents: false } }),
        ],
        // Tracing：20% 采样，兼顾趋势与上报量
        tracesSampleRate: 0.2,
        // Set 'tracePropagationTargets' to control for which URLs distributed tracing should be enabled
        // tracePropagationTargets: ['localhost'],
        // Session Replay
        replaysSessionSampleRate: 0.05,
        // 出错时仍 100% 录制，便于排障
        replaysOnErrorSampleRate: 1.0,
        release: `quicklook-vue@${pkg.version}`,
    })
}
