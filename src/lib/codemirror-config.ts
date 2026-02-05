import { markdown, markdownLanguage } from "@codemirror/lang-markdown";
import { yamlFrontmatter } from "@codemirror/lang-yaml";
import { EditorView } from "@uiw/react-codemirror";

export const codeMirrorBasicSetup = {
	lineNumbers: false,
	highlightActiveLineGutter: true,
	foldGutter: false,
	dropCursor: false,
	allowMultipleSelections: false,
	indentOnInput: true,
	bracketMatching: true,
	closeBrackets: true,
	autocompletion: true,
	highlightActiveLine: true,
	highlightSelectionMatches: true,
	searchKeymap: false,
} as const;

export const markdownExtensions = [
	yamlFrontmatter({
		content: markdown({
			base: markdownLanguage,
		}),
	}),
	EditorView.lineWrapping,
];
