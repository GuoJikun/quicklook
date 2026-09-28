import { init, browserTracingIntegration, vueIntegration } from '@sentry/vue'
import type { App } from 'vue'
import type { Router } from 'vue-router'
import pkg from '../../package.json'

export interface SentryConfig {
    app: App
    router: Router
}

export default (conf: SentryConfig) => {
    init({
        app: conf.app,
        dsn: 'https://8fb284d3b45b3f203d904cabf2460287@o4507727722905600.ingest.us.sentry.io/4508803797286912',
        integrations: [
            browserTracingIntegration({ router: conf.router }),
            vueIntegration({ tracingOptions: { trackComponents: false } }),
        ],
        // Tracing：20% 采样，兼顾趋势与上报量
        tracesSampleRate: 0.2,
        // Set 'tracePropagationTargets' to control for which URLs distributed tracing should be enabled
        // tracePropagationTargets: ['localhost'],
        release: `quicklook-vue@${pkg.version}`,
    })
}
