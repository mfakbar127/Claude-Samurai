import { ask } from "@tauri-apps/plugin-dialog";
import CodeMirror from "@uiw/react-codemirror";
import { SaveIcon, SparklesIcon, TrashIcon } from "lucide-react";
import { Suspense, useState } from "react";
import { useTranslation } from "react-i18next";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
} from "@/components/ui/accordion";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Skeleton } from "@/components/ui/skeleton";
import {
	type MemoryEntry,
	useClaudeMemoryFiles,
	useDeleteClaudeMemoryFile,
	useToggleClaudeMemoryFile,
	useWriteClaudeMemoryFile,
} from "@/lib/query";
import { codeMirrorBasicSetup, markdownExtensions } from "@/lib/codemirror-config";
import { useCodeMirrorTheme } from "@/lib/use-codemirror-theme";

function MemoryPageHeader() {
	const { t } = useTranslation();

	return (
		<div
			className="flex items-center justify-between sticky top-0 z-10 border-b p-3 bg-background"
			data-tauri-drag-region
		>
			<div data-tauri-drag-region>
				<h3 className="font-bold" data-tauri-drag-region>
					{t("memory.title")}
				</h3>
				<p className="text-sm text-muted-foreground" data-tauri-drag-region>
					{t("memory.description")}
				</p>
			</div>
		</div>
	);
}

function MemoryPageSkeleton() {
	return (
		<div className="flex flex-col h-screen">
			<div
				className="flex items-center p-3 border-b px-3 justify-between sticky top-0 bg-background z-10"
				data-tauri-drag-region
			>
				<div data-tauri-drag-region>
					<Skeleton className="h-6 w-16 mb-2" />
					<Skeleton className="h-4 w-64" />
				</div>
				<Skeleton className="h-8 w-16" />
			</div>
			<div className="flex-1 p-4 overflow-hidden">
				<div className="rounded-lg overflow-hidden border h-full">
					<div className="h-full flex items-center justify-center">
						<div className="space-y-2 w-full max-w-2xl">
							<Skeleton className="h-4 w-full" />
							<Skeleton className="h-4 w-3/4" />
							<Skeleton className="h-4 w-1/2" />
							<Skeleton className="h-4 w-full" />
							<Skeleton className="h-4 w-2/3" />
						</div>
					</div>
				</div>
			</div>
		</div>
	);
}

function MemoryPageContent() {
	const { t } = useTranslation();
	const { data: memories, isLoading, error } = useClaudeMemoryFiles();
	const writeMemory = useWriteClaudeMemoryFile();
	const toggleMemory = useToggleClaudeMemoryFile();
	const deleteMemory = useDeleteClaudeMemoryFile();
	const [edits, setEdits] = useState<Record<string, string>>({});
	const codeMirrorTheme = useCodeMirrorTheme();

	if (isLoading) {
		return (
			<div className="flex items-center justify-center min-h-screen">
				<div className="text-center">{t("loading")}</div>
			</div>
		);
	}

	if (error) {
		return (
			<div className="flex items-center justify-center min-h-screen">
				<div className="text-center text-red-500">
					{t("memory.error", { error: (error as Error).message })}
				</div>
			</div>
		);
	}

	const memoryList = (memories ?? []).slice();

	const getEntryKey = (entry: MemoryEntry) =>
		entry.source === "global" ? "global" : entry.projectPath ?? entry.name;

	const getProjectDisplayName = (projectPath?: string, fallback?: string) => {
		if (!projectPath) return fallback ?? "";
		const parts = projectPath.split(/[/\\]/).filter(Boolean);
		return parts[parts.length - 1] ?? projectPath;
	};

	const handleSaveMemory = (entry: MemoryEntry) => {
		const key = getEntryKey(entry);
		const content = edits[key] ?? entry.content;

		writeMemory.mutate({
			source: entry.source,
			projectPath: entry.projectPath,
			content,
			disabled: entry.disabled,
		});
	};

	const handleToggleMemory = (entry: MemoryEntry) => {
		toggleMemory.mutate({
			source: entry.source,
			projectPath: entry.projectPath,
			disabled: !entry.disabled,
		});
	};

	const handleDeleteMemory = async (entry: MemoryEntry) => {
		const label =
			entry.source === "global"
				? t("memory.globalLabel")
				: t("memory.projectLabel", {
						path: getProjectDisplayName(entry.projectPath, entry.name),
					});

		const confirmed = await ask(
			t("memory.deleteConfirm", { label }),
			{
				title: t("memory.deleteTitle"),
				kind: "warning",
			},
		);

		if (!confirmed) {
			return;
		}

		deleteMemory.mutate({
			source: entry.source,
			projectPath: entry.projectPath,
		});
	};

	return (
		<div>
			<MemoryPageHeader />
			<div>
				{memoryList.length === 0 ? (
					<div className="text-center text-muted-foreground py-8">
						{t("memory.noMemories")}
					</div>
				) : (
					<ScrollArea className="h-full">
						<div>
							<Accordion type="multiple">
								{memoryList.map((entry) => {
									const key = getEntryKey(entry);
									const label =
										entry.source === "global"
											? t("memory.globalLabel")
											: t("memory.projectLabel", {
													path: getProjectDisplayName(
														entry.projectPath,
														entry.name,
													),
												});

									return (
										<AccordionItem
											key={key}
											value={key}
											className="bg-card"
										>
											<AccordionTrigger className="hover:no-underline px-4 py-2 bg-card hover:bg-accent duration-150">
												<div className="flex items-center justify-between gap-2 w-full">
													<div className="flex items-center gap-2 flex-wrap">
														<SparklesIcon size={12} />
														<span className="font-medium">
															{label}
														</span>
														<Badge
															variant={
																entry.disabled
																	? "destructive"
																	: "success"
															}
														>
															{entry.disabled
																? t("memory.disabled")
																: t("memory.enabled")}
														</Badge>
														{entry.source === "project" &&
															entry.projectPath && (
																<span className="text-xs text-muted-foreground font-mono truncate max-w-xs">
																	{entry.projectPath}
																</span>
															)}
													</div>
													<div className="flex items-center gap-2">
														{entry.source === "global" && (
															<Badge variant="secondary">
																{t("memory.sourceGlobal")}
															</Badge>
														)}
														{entry.source === "project" && (
															<Badge variant="secondary">
																{t("memory.sourceProject")}
															</Badge>
														)}
													</div>
												</div>
											</AccordionTrigger>
											<AccordionContent className="pb-3">
												<div className="px-3 pt-3 space-y-3">
													<div className="rounded-lg overflow-hidden border">
														<CodeMirror
															value={
																edits[key] ?? entry.content
															}
															height="280px"
															theme={codeMirrorTheme}
															onChange={(value) =>
																setEdits((prev) => ({
																	...prev,
																	[key]: value,
																}))
															}
															placeholder={t(
																"memory.contentPlaceholder",
															)}
															extensions={markdownExtensions}
															basicSetup={codeMirrorBasicSetup}
														/>
													</div>
													<div className="flex justify-between bg-card px-1 py-1">
														<div className="flex items-center text-xs text-muted-foreground font-mono">
															<span className="truncate max-w-xs">
																{entry.path}
															</span>
														</div>
														<div className="flex gap-2">
															<Button
																variant="outline"
																size="sm"
																onClick={() =>
																	handleSaveMemory(entry)
																}
																disabled={
																	writeMemory.isPending
																}
															>
																<SaveIcon
																	size={12}
																	className="mr-1"
																/>
																{writeMemory.isPending
																	? t("memory.saving")
																	: t("memory.save")}
															</Button>
															<Button
																variant="outline"
																size="sm"
																onClick={() =>
																	handleToggleMemory(entry)
																}
																disabled={
																	toggleMemory.isPending
																}
															>
																{entry.disabled
																	? t("memory.enable")
																	: t("memory.disable")}
															</Button>
															<Button
																variant="outline"
																size="sm"
																onClick={() => {
																	void handleDeleteMemory(
																		entry,
																	);
																}}
																disabled={
																	deleteMemory.isPending
																}
															>
																<TrashIcon
																	size={12}
																	className="mr-1"
																/>
																{t("memory.delete")}
															</Button>
														</div>
													</div>
												</div>
											</AccordionContent>
										</AccordionItem>
									);
								})}
							</Accordion>
						</div>
					</ScrollArea>
				)}
			</div>
		</div>
	);
}

export function MemoryPage() {
	return (
		<Suspense fallback={<MemoryPageSkeleton />}>
			<MemoryPageContent />
		</Suspense>
	);
}
