/* eslint-disable @typescript-eslint/ban-ts-comment */
import MarkdownIt from 'markdown-it'
import Shiki from '@shikijs/markdown-it'
// @ts-expect-error
import markdownItAbbr from 'markdown-it-abbr'
// @ts-expect-error
import markdownItDeflist from 'markdown-it-deflist'
// @ts-expect-error
import { full as markdownItEmoji } from 'markdown-it-emoji'
// @ts-expect-error
import markdownItIns from 'markdown-it-ins'
// @ts-expect-error
import markdownItTaskLists from 'markdown-it-task-lists'
// @ts-expect-error
import markdownItTocAndAnchor from 'markdown-it-toc-and-anchor'
// @ts-expect-error
import markdownItKatex from 'markdown-it-katex'
// @ts-expect-error
import markdownItTableOfContents from 'markdown-it-table-of-contents'

import containerPlugin from './plugins/container'
import { preWrapperPlugin } from './plugins/preWrapper'

const createMd = async (): Promise<MarkdownIt> => {
    const md = new MarkdownIt()

    const highlight = await Shiki({
        themes: {
            light: 'github-light',
            dark: 'github-dark',
        },
        defaultColor: 'light-dark()',
    })

    md.use(markdownItAbbr)
    md.use(markdownItDeflist)
    md.use(markdownItEmoji)
    md.use(markdownItIns)
    md.use(markdownItTaskLists)
    md.use(markdownItTocAndAnchor)
    containerPlugin(md)
    preWrapperPlugin(md, { codeCopyButtonTitle: '复制', hasSingleTheme: true })
    md.use(markdownItKatex)

    md.use(markdownItTableOfContents)

    md.use(highlight)
    return md
}

export default createMd
export { createMd }
