import { json } from "@codemirror/lang-json";
import CodeMirror from "@uiw/react-codemirror";
import { SparklesIcon } from "lucide-react";
import { Suspense } from "react";
import { useTranslation } from "react-i18next";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
} from "@/components/ui/accordion";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
	type HooksConfigEntry,
	useHooksSettings,
} from "@/lib/query";
import { codeMirrorBasicSetup } from "@/lib/codemirror-config";
import { useCodeMirrorTheme } from "@/lib/use-codemirror-theme";

const hooksSourceOrder = {
	project_local: 0,
	project: 1,
	user: 2,
} satisfies Record<HooksConfigEntry["source"], number>;

function hasHooks(entry: HooksConfigEntry): entry is HooksConfigEntry & { hooks: NonNullable<HooksConfigEntry["hooks"]> } {
	return Boolean(entry.hooks);
}

function getSourceLabel(
	t: (key: string) => string,
	source: HooksConfigEntry["source"],
): string {
	if (source === "project_local") {
		return t("hooks.sourceProjectLocal");
	}
	if (source === "project") {
		return t("hooks.sourceProject");
	}
	return t("hooks.sourceUser");
}

function getProjectNameFromPath(path: string): string {
	const parts = path.split(/[/\\]/).filter(Boolean);
	const claudeIndex = parts.lastIndexOf(".claude");
	if (claudeIndex > 0) {
		return parts[claudeIndex - 1] ?? parts[claudeIndex] ?? path;
	}
	return parts[parts.length - 1] ?? path;
}

function getEntryTitle(t: (key: string) => string, entry: HooksConfigEntry): string {
	if (entry.source === "user") {
		return t("hooks.sourceUser");
	}

	const projectName = getProjectNameFromPath(entry.path);

	if (entry.source === "project_local") {
		return `${t("hooks.sourceProjectLocal")} – ${projectName}`;
	}

	if (entry.source === "project") {
		return `${t("hooks.sourceProject")} – ${projectName}`;
	}

	return getSourceLabel(t, entry.source);
}

function formatHooks(t: (key: string, options?: Record<string, unknown>) => string, entry: HooksConfigEntry): string {
	if (!entry.exists) {
		return t("hooks.noFile");
	}
	if (entry.error) {
		return t("hooks.readError", { error: entry.error });
	}
	if (!entry.hooks) {
		return t("hooks.noHooksBody");
	}
	try {
		return JSON.stringify(entry.hooks, null, 2);
	} catch {
		return t("hooks.invalidJson");
	}
}

function HooksPageContent() {
	const { t } = useTranslation();
	const { data: hooksEntries } = useHooksSettings();
	const codeMirrorTheme = useCodeMirrorTheme();

	const entriesWithHooks = (hooksEntries ?? []).filter(hasHooks);

	const sortedEntries = [...entriesWithHooks].sort((a, b) => {
		const aOrder = hooksSourceOrder[a.source];
		const bOrder = hooksSourceOrder[b.source];
		if (aOrder !== bOrder) return aOrder - bOrder;
		return a.path.localeCompare(b.path);
	});

	return (
		<div>
			<div
				className="flex items-center justify-between sticky top-0 z-10 border-b p-3 bg-background"
				data-tauri-drag-region
			>
				<div data-tauri-drag-region>
					<h3 className="font-bold" data-tauri-drag-region>
						{t("hooks.title")}
					</h3>
					<p className="text-sm text-muted-foreground" data-tauri-drag-region>
						{t("hooks.description")}
					</p>
				</div>
			</div>
			<div>
				{sortedEntries.length === 0 ? (
					<div className="text-center text-muted-foreground py-8">
						{t("hooks.noEntries")}
					</div>
				) : (
					<ScrollArea className="h-full">
						<div>
							<Accordion type="multiple">
								{sortedEntries.map((entry) => (
									<AccordionItem
										key={`${entry.source}-${entry.path}`}
										value={`${entry.source}-${entry.path}`}
										className="bg-card"
									>
										<AccordionTrigger className="hover:no-underline px-4 py-2 bg-card hover:bg-accent duration-150">
											<div className="flex items-center justify-between gap-2 w-full">
												<div className="flex items-center gap-2 flex-wrap">
													<SparklesIcon size={12} />
													<span className="font-medium">
														{getEntryTitle(t, entry)}
													</span>
													<span className="text-xs text-muted-foreground font-mono truncate max-w-xs">
														{entry.path}
													</span>
												</div>
												<div className="flex items-center gap-2">
													{entry.source === "user" && (
														<Badge variant="secondary">
															{t("hooks.sourceUser")}
														</Badge>
													)}
													{entry.source === "project" && (
														<Badge variant="secondary">
															{t("hooks.sourceProject")}
														</Badge>
													)}
													{entry.source === "project_local" && (
														<Badge variant="secondary">
															{t("hooks.sourceProjectLocal")}
														</Badge>
													)}
												</div>
											</div>
										</AccordionTrigger>
										<AccordionContent className="pb-3">
											<div className="px-3 pt-3 space-y-3">
												<div className="rounded-lg overflow-hidden border">
													<CodeMirror
														value={formatHooks(t, entry)}
														height="280px"
														theme={codeMirrorTheme}
														extensions={[json()]}
														basicSetup={codeMirrorBasicSetup}
														readOnly
													/>
												</div>
												<div className="flex justify-between bg-card px-1 py-1">
													<div className="flex items-center text-xs text-muted-foreground font-mono">
														<span className="truncate max-w-xs">
															{entry.path}
														</span>
													</div>
												</div>
											</div>
										</AccordionContent>
									</AccordionItem>
								))}
							</Accordion>
						</div>
					</ScrollArea>
				)}
			</div>
		</div>
	);
}

export function HooksPage() {
	const { t } = useTranslation();

	return (
		<Suspense
			fallback={
				<div className="flex items-center justify-center min-h-screen">
					<div className="text-center">{t("loading")}</div>
				</div>
			}
		>
			<HooksPageContent />
		</Suspense>
	);
}

