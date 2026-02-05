import { ask, message } from "@tauri-apps/plugin-dialog";
import CodeMirror from "@uiw/react-codemirror";
import { BotIcon, PlusIcon, SaveIcon, TrashIcon } from "lucide-react";
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
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
	DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ScrollArea } from "@/components/ui/scroll-area";
import { codeMirrorBasicSetup, markdownExtensions } from "@/lib/codemirror-config";
import {
	useClaudeAgents,
	useDeleteClaudeAgent,
	usePluginAgents,
	useWriteClaudeAgent,
} from "@/lib/query";
import { useCodeMirrorTheme } from "@/lib/use-codemirror-theme";

type UnifiedAgent = {
	name: string;
	content: string;
	exists: boolean;
	source: "user" | "plugin";
	pluginName?: string;
	pluginScope?: string;
	sourcePath: string;
};

function AgentsPageContent() {
	const { t } = useTranslation();
	const { data: userAgents, isLoading: isLoadingUser, error: errorUser } = useClaudeAgents();
	const { data: pluginAgents, isLoading: isLoadingPlugin, error: errorPlugin } = usePluginAgents();
	const writeAgent = useWriteClaudeAgent();
	const deleteAgent = useDeleteClaudeAgent();
	const [agentEdits, setAgentEdits] = useState<Record<string, string>>({});
	const [isDialogOpen, setIsDialogOpen] = useState(false);
	const codeMirrorTheme = useCodeMirrorTheme();

	const isLoading = isLoadingUser || isLoadingPlugin;
	const error = errorUser || errorPlugin;

	const agents: UnifiedAgent[] = [
		...(userAgents || []).map((agent): UnifiedAgent => ({
			name: agent.name,
			content: agent.content,
			exists: agent.exists,
			source: "user",
			sourcePath: `~/.claude/agents/${agent.name}.md`,
		})),
		...(pluginAgents || []).map((agent): UnifiedAgent => ({
			name: agent.name,
			content: agent.content,
			exists: agent.exists,
			source: "plugin",
			pluginName: agent.pluginName,
			pluginScope: agent.pluginScope,
			sourcePath: agent.sourcePath,
		})),
	].sort((a, b) => a.name.localeCompare(b.name));

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
					{t("agents.error", { error: error.message })}
				</div>
			</div>
		);
	}

	const handleContentChange = (agentName: string, content: string) => {
		setAgentEdits((prev) => ({
			...prev,
			[agentName]: content,
		}));
	};

	const handleSaveAgent = async (agentName: string) => {
		const content = agentEdits[agentName];
		if (content === undefined) return;

		writeAgent.mutate({
			agentName,
			content,
		});
	};

	const handleDeleteAgent = async (agentName: string) => {
		const confirmed = await ask(t("agents.deleteConfirm", { agentName }), {
			title: t("agents.deleteTitle"),
			kind: "warning",
		});

		if (confirmed) {
			deleteAgent.mutate(agentName);
		}
	};

	return (
		<div>
			<div
				className="flex items-center justify-between sticky top-0 z-10 border-b p-3 bg-background"
				data-tauri-drag-region
			>
				<div data-tauri-drag-region>
					<h3 className="font-bold" data-tauri-drag-region>
						{t("agents.title")}
					</h3>
					<p className="text-sm text-muted-foreground" data-tauri-drag-region>
						{t("agents.description")}
					</p>
				</div>
				<Dialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
					<DialogTrigger asChild>
						<Button variant="ghost" className="text-muted-foreground" size="sm">
							<PlusIcon size={14} />
							{t("agents.addAgent")}
						</Button>
					</DialogTrigger>
					<DialogContent className="max-w-[600px]">
						<DialogHeader>
							<DialogTitle>{t("agents.addAgentTitle")}</DialogTitle>
							<DialogDescription className="text-muted-foreground text-sm">
								{t("agents.addAgentDescription")}
							</DialogDescription>
						</DialogHeader>
						<CreateAgentPanel onClose={() => setIsDialogOpen(false)} />
					</DialogContent>
				</Dialog>
			</div>
			<div>
				{agents.length === 0 ? (
					<div className="text-center text-muted-foreground py-8">
						{t("agents.noAgents")}
					</div>
				) : (
					<ScrollArea className="h-full">
						<div>
							<Accordion type="multiple">
								{agents.map((agent) => (
									<AccordionItem
										key={agent.name}
										value={agent.name}
										className="bg-card"
									>
										<AccordionTrigger className="hover:no-underline px-4 py-2 bg-card hover:bg-accent duration-150">
											<div className="flex items-center gap-2 flex-wrap">
												<BotIcon size={12} />
												<span className="font-medium">{agent.name}</span>
												{agent.source === "user" ? (
													<Badge variant="default">
														{t("agents.sourceUser")}
													</Badge>
												) : (
													<>
														<Badge variant="outline">
															{agent.pluginName}
														</Badge>
														<Badge variant={agent.pluginScope === "user" ? "default" : "secondary"}>
															{agent.pluginScope === "user" ? t("plugins.scopeUser") : t("plugins.scopeLocal")}
														</Badge>
													</>
												)}
												<span className="text-sm text-muted-foreground font-normal">
													{agent.sourcePath}
												</span>
											</div>
										</AccordionTrigger>
										<AccordionContent className="pb-3">
											<div className="px-3 pt-3 space-y-3">
												<div className="rounded-lg overflow-hidden border">
													<CodeMirror
														value={agentEdits[agent.name] ?? agent.content}
														height="180px"
														theme={codeMirrorTheme}
														onChange={(value) =>
															handleContentChange(agent.name, value)
														}
														placeholder={t("agents.contentPlaceholder")}
														extensions={markdownExtensions}
														basicSetup={codeMirrorBasicSetup}
													/>
												</div>
												<div className="flex justify-between bg-card">
													<Button
														variant="outline"
														onClick={() => handleSaveAgent(agent.name)}
														disabled={
															writeAgent.isPending ||
															agentEdits[agent.name] === undefined
														}
														size="sm"
													>
														<SaveIcon size={14} />
														{writeAgent.isPending
															? t("agents.saving")
															: t("agents.save")}
													</Button>

													<Button
														variant="ghost"
														size="sm"
														onClick={() => handleDeleteAgent(agent.name)}
														disabled={deleteAgent.isPending}
													>
														<TrashIcon size={14} />
													</Button>
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

export function AgentsPage() {
	const { t } = useTranslation();

	return (
		<Suspense
			fallback={
				<div className="flex items-center justify-center min-h-screen">
					<div className="text-center">{t("loading")}</div>
				</div>
			}
		>
			<AgentsPageContent />
		</Suspense>
	);
}

type CreateAgentPanelProps = {
	onClose?: () => void;
};

function CreateAgentPanel({ onClose }: CreateAgentPanelProps) {
	const { t } = useTranslation();
	const [agentName, setAgentName] = useState("");
	const [agentContent, setAgentContent] = useState(`---
name: your-sub-agent-name
description: Description of when this subagent should be invoked
tools: tool1, tool2, tool3  # Optional - inherits all tools if omitted
model: sonnet  # Optional - specify model alias or 'inherit'
---

Your subagent's system prompt goes here. This can be multiple paragraphs
and should clearly define the subagent's role, capabilities, and approach
to solving problems.

Include specific instructions, best practices, and any constraints
the subagent should follow.`);
	const writeAgent = useWriteClaudeAgent();
	const { data: agents } = useClaudeAgents();
	const codeMirrorTheme = useCodeMirrorTheme();

	const handleCreateAgent = async () => {
		if (!agentName.trim()) {
			await message(t("agents.emptyNameError"), {
				title: t("agents.validationError"),
				kind: "error",
			});
			return;
		}

		if (agents?.some((a) => a.name === agentName)) {
			await message(t("agents.agentExistsError", { agentName }), {
				title: t("agents.agentExistsTitle"),
				kind: "info",
			});
			return;
		}

		if (!agentContent.trim()) {
			await message(t("agents.emptyContentError"), {
				title: t("agents.validationError"),
				kind: "error",
			});
			return;
		}

		writeAgent.mutate(
			{
				agentName,
				content: agentContent,
			},
			{
				onSuccess: () => {
					setAgentName("");
					setAgentContent("");
					onClose?.();
				},
			},
		);
	};

	return (
		<div className="space-y-4 mt-4">
			<div className="space-y-2">
				<Label className="block" htmlFor="agent-name">
					{t("agents.agentName")}
				</Label>
				<Input
					id="agent-name"
					value={agentName}
					onChange={(e) => setAgentName(e.target.value)}
					placeholder={t("agents.agentNamePlaceholder")}
				/>
			</div>

			<div className="space-y-2">
				<Label className="block" htmlFor="agent-content">
					{t("agents.agentContent")}
				</Label>
				<div className="rounded-lg overflow-hidden border">
					<CodeMirror
						value={agentContent}
						onChange={(value) => setAgentContent(value)}
						height="200px"
						theme={codeMirrorTheme}
														placeholder={t("agents.contentPlaceholder")}
														extensions={markdownExtensions}
														basicSetup={codeMirrorBasicSetup}
					/>
				</div>
			</div>

			<div className="flex justify-end">
				<Button
					onClick={handleCreateAgent}
					disabled={
						!agentName.trim() || !agentContent.trim() || writeAgent.isPending
					}
				>
					{writeAgent.isPending ? t("agents.creating") : t("agents.create")}
				</Button>
			</div>
		</div>
	);
}
