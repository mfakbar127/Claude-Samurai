import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
	Dialog,
	DialogContent,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Separator } from "@/components/ui/separator";
import { json } from "@codemirror/lang-json";
import CodeMirror from "@uiw/react-codemirror";
import { useCodeMirrorTheme } from "@/lib/use-codemirror-theme";
import {
	type InstalledSecurityPackItem,
	type McpTemplate,
	type SecurityPackType,
	type SecurityTemplates,
	useInstallSecurityTemplate,
	useInstalledSecurityTemplates,
	useSecurityTemplates,
	useUninstallSecurityTemplate,
} from "@/lib/query";
import { codeMirrorBasicSetup } from "@/lib/codemirror-config";

// Static asset globs for pack contents
const PACKS_BASE =
	"../assets/security_packs/security_templates_packs/" as const;

const agentSources = import.meta.glob(
	"../assets/security_packs/security_templates_packs/agents/*.md",
	{ as: "raw", eager: true },
) as Record<string, string>;

const commandSources = import.meta.glob(
	"../assets/security_packs/security_templates_packs/commands/*.md",
	{ as: "raw", eager: true },
) as Record<string, string>;

const skillSources = import.meta.glob(
	"../assets/security_packs/security_templates_packs/skills/**/*",
	{ as: "raw", eager: true },
) as Record<string, string>;

type TemplateItem =
	| (SecurityTemplates["agents"][number] & { type: "agent" })
	| (SecurityTemplates["skills"][number] & { type: "skill" })
	| (SecurityTemplates["commands"][number] & { type: "command" })
	| (McpTemplate & { type: "mcp" });

interface DetailState {
	item: TemplateItem;
	installed: boolean;
}

function getAgentContent(sourcePath: string): string | undefined {
	const key = `${PACKS_BASE}${sourcePath}`;
	return agentSources[key];
}

function getCommandContent(sourcePath: string): string | undefined {
	const key = `${PACKS_BASE}${sourcePath}`;
	return commandSources[key];
}

function getSkillFilesForId(
	skillId: string,
): { relativePath: string; content: string }[] {
	const prefix = `${PACKS_BASE}skills/${skillId}/`;
	const entries: { relativePath: string; content: string }[] = [];

	for (const [path, content] of Object.entries(skillSources)) {
		if (path.startsWith(prefix)) {
			const relativePath = path.slice(prefix.length);
			if (!relativePath) continue;
			entries.push({ relativePath, content });
		}
	}

	return entries;
}

function getMarkdownForTemplate(item: TemplateItem): string | undefined {
	if (item.type === "mcp") {
		return undefined;
	}

	if (item.type === "agent") {
		const agentItem = item as SecurityTemplates["agents"][number] & {
			type: "agent";
		};
		return getAgentContent(agentItem.sourcePath);
	}

	if (item.type === "command") {
		const commandItem = item as SecurityTemplates["commands"][number] & {
			type: "command";
		};
		return getCommandContent(commandItem.sourcePath);
	}

	// For skills, prefer SKILL.md content
	if (item.type === "skill") {
		const skillItem = item as SecurityTemplates["skills"][number] & {
			type: "skill";
		};
		const key = `${PACKS_BASE}${skillItem.sourcePath}`;
		return skillSources[key];
	}

	return undefined;
}

function sectionLabel(t: (key: string) => string, type: SecurityPackType): string {
	switch (type) {
		case "agent":
			return "Agents";
		case "skill":
			return t("navigation.skills");
		case "command":
			return t("navigation.commands");
		case "mcp":
			return t("navigation.mcp");
	}
}

function SecurityPacksSection(props: {
	title: string;
	items: TemplateItem[];
	installedMap: Map<string, InstalledSecurityPackItem>;
	onShowDetails: (item: TemplateItem, installed: boolean) => void;
	onToggleInstall: (item: TemplateItem, installed: boolean) => void;
}) {
	if (props.items.length === 0) {
		return null;
	}

	return (
		<section className="space-y-3">
			<div className="flex items-center justify-between">
				<h3 className="text-sm font-semibold tracking-tight text-muted-foreground">
					{props.title}
				</h3>
				<Separator className="flex-1 ml-3" />
			</div>
			<div className="space-y-2">
				{props.items.map((item) => {
					const installed = props.installedMap.has(
						`${item.type}:${item.id}`,
					);

					return (
						<Card key={`${item.type}-${item.id}`} className="bg-card">
							<CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
								<div>
									<CardTitle className="text-sm font-medium">
										{item.title}
									</CardTitle>
									<p className="text-xs text-muted-foreground">
										{item.description}
									</p>
								</div>
								<div className="flex items-center gap-2">
									<Badge variant={installed ? "default" : "outline"}>
										{installed ? "Installed" : "Not installed"}
									</Badge>
								</div>
							</CardHeader>
							<CardContent className="flex items-center justify-between pt-2 gap-2">
								<div className="text-xs text-muted-foreground">
									{item.type === "mcp" && (
										<span>{(item as McpTemplate & { type: "mcp" }).serverName}</span>
									)}
								</div>
								<div className="flex items-center gap-2">
									<Button
										size="sm"
										variant="outline"
										onClick={() =>
											props.onShowDetails(item, installed)
										}
									>
										{item.type === "mcp"
											? "View config"
											: "View markdown"}
									</Button>
									<Button
										size="sm"
										variant={installed ? "outline" : "default"}
										onClick={() =>
											props.onToggleInstall(item, installed)
										}
									>
										{installed ? "Uninstall" : "Install"}
									</Button>
								</div>
							</CardContent>
						</Card>
					);
				})}
			</div>
		</section>
	);
}

export function SecurityPacksPage() {
	const { t } = useTranslation();
	const { data: templates } = useSecurityTemplates();
	const { data: installedItems } = useInstalledSecurityTemplates();
	const installMutation = useInstallSecurityTemplate();
	const uninstallMutation = useUninstallSecurityTemplate();
	const [detail, setDetail] = useState<DetailState | null>(null);
	const codeMirrorTheme = useCodeMirrorTheme();
	const [selectedType, setSelectedType] = useState<SecurityPackType>("agent");

	const installedMap = useMemo(() => {
		const map = new Map<string, InstalledSecurityPackItem>();
		for (const item of installedItems ?? []) {
			map.set(`${item.type}:${item.id}`, item);
		}
		return map;
	}, [installedItems]);

	const agentItems: TemplateItem[] =
		templates?.agents.map((a) => ({ ...a, type: "agent" })) ?? [];
	const skillItems: TemplateItem[] =
		templates?.skills.map((s) => ({ ...s, type: "skill" })) ?? [];
	const commandItems: TemplateItem[] =
		templates?.commands.map((c) => ({ ...c, type: "command" })) ?? [];
	const mcpItems: TemplateItem[] =
		(templates?.mcp as McpTemplate[] | undefined)?.map((m) => ({
			...m,
			type: "mcp" as const,
		})) ?? [];

	const itemsByType: Record<SecurityPackType, TemplateItem[]> = {
		agent: agentItems,
		skill: skillItems,
		command: commandItems,
		mcp: mcpItems,
	};

	const selectedItems: TemplateItem[] = itemsByType[selectedType];

	function handleInstallUninstall(
		item: TemplateItem,
		installed: boolean,
		closeDialog?: boolean,
	): void {
		if (installed) {
			const uninstallId =
				item.type === "mcp" ? (item as McpTemplate).serverName : item.id;
			uninstallMutation.mutate({
				type: item.type,
				id: uninstallId,
			});
			if (closeDialog) {
				setDetail(null);
			}
			return;
		}

		switch (item.type) {
			case "mcp": {
				const mcpItem = item as McpTemplate;
				installMutation.mutate({
					type: "mcp",
					id: mcpItem.id,
					serverName: mcpItem.serverName,
					serverConfig: mcpItem.serverConfig,
				});
				if (closeDialog) {
					setDetail(null);
				}
				break;
			}
			case "agent":
			case "command": {
				const markdown = getMarkdownForTemplate(item);
				if (!markdown) {
					return;
				}
				installMutation.mutate({
					type: item.type,
					id: item.id,
					content: markdown,
				});
				if (closeDialog) {
					setDetail(null);
				}
				break;
			}
			case "skill": {
				const files = getSkillFilesForId(item.id);
				if (files.length === 0) {
					return;
				}
				installMutation.mutate({
					type: "skill",
					id: item.id,
					skillFiles: files,
				});
				if (closeDialog) {
					setDetail(null);
				}
				break;
			}
		}
	}

	function handleToggle(item: TemplateItem, installed: boolean): void {
		handleInstallUninstall(item, installed, false);
	}

	function renderDetailBody(current: DetailState) {
		if (current.item.type === "mcp") {
			const mcpItem = current.item as McpTemplate & { type: "mcp" };
			return (
				<div className="space-y-3">
					<p className="text-xs text-muted-foreground">
						{mcpItem.description}
					</p>
					<div className="border rounded-md overflow-hidden">
						<CodeMirror
							value={JSON.stringify(mcpItem.serverConfig, null, 2)}
							height="260px"
							theme={codeMirrorTheme}
							extensions={[json()]}
							basicSetup={codeMirrorBasicSetup}
							readOnly
						/>
					</div>
				</div>
			);
		}

		const markdown = getMarkdownForTemplate(current.item);

		if (!markdown) {
			return (
				<p className="text-xs text-muted-foreground">
					{current.item.description}
				</p>
			);
		}

		return (
			<div className="space-y-3">
				<p className="text-xs text-muted-foreground">
					{current.item.description}
				</p>
				<div className="border rounded-md bg-card p-2">
					<pre className="whitespace-pre-wrap text-xs text-foreground">
						{markdown}
					</pre>
				</div>
			</div>
		);
	}

	function handleDetailPrimary(current: DetailState): void {
		handleInstallUninstall(current.item, current.installed, true);
	}

	return (
		<div className="flex flex-col h-full">
			<div
				className="flex items-center justify-between sticky top-0 z-10 border-b p-3 bg-background"
				data-tauri-drag-region
			>
				<div data-tauri-drag-region>
					<h3 className="font-bold" data-tauri-drag-region>
						Security Packs
					</h3>
					<p
						className="text-sm text-muted-foreground"
						data-tauri-drag-region
					>
						Install curated security-focused agents, skills, commands, and
						MCP servers.
					</p>
				</div>
			</div>
			<div className="p-3 border-b bg-muted/30">
				<div className="flex gap-1">
					<Button
						size="sm"
						variant={selectedType === "agent" ? "secondary" : "ghost"}
						className="text-xs"
						onClick={() => setSelectedType("agent")}
					>
						Agents
					</Button>
					<Button
						size="sm"
						variant={selectedType === "skill" ? "secondary" : "ghost"}
						className="text-xs"
						onClick={() => setSelectedType("skill")}
					>
						{t("navigation.skills")}
					</Button>
					<Button
						size="sm"
						variant={selectedType === "command" ? "secondary" : "ghost"}
						className="text-xs"
						onClick={() => setSelectedType("command")}
					>
						{t("navigation.commands")}
					</Button>
					<Button
						size="sm"
						variant={selectedType === "mcp" ? "secondary" : "ghost"}
						className="text-xs"
						onClick={() => setSelectedType("mcp")}
					>
						{t("navigation.mcp")}
					</Button>
				</div>
			</div>
			<ScrollArea className="flex-1 h-full">
				<div className="p-3 space-y-6">
					<SecurityPacksSection
						title={sectionLabel(t, selectedType)}
						items={selectedItems}
						installedMap={installedMap}
						onShowDetails={(item, installed) =>
							setDetail({ item, installed })
						}
						onToggleInstall={handleToggle}
					/>
				</div>
			</ScrollArea>

			<Dialog open={detail !== null} onOpenChange={() => setDetail(null)}>
				<DialogContent className="max-w-xl" aria-describedby={undefined}>
					{detail && (
						<>
							<DialogHeader>
								<DialogTitle className="flex items-center justify-between gap-2">
									<span>{detail.item.title}</span>
									<Badge
										variant={
											detail.installed ? "default" : "outline"
										}
									>
										{detail.installed
											? "Installed"
											: "Not installed"}
									</Badge>
								</DialogTitle>
							</DialogHeader>
							{renderDetailBody(detail)}
							<DialogFooter className="flex justify-between">
								<Button
									variant="outline"
									onClick={() => setDetail(null)}
								>
									Close
								</Button>
								<Button
									variant={
										detail.installed ? "outline" : "default"
									}
									onClick={() => handleDetailPrimary(detail)}
								>
									{detail.installed ? "Uninstall" : "Install"}
								</Button>
							</DialogFooter>
						</>
					)}
				</DialogContent>
			</Dialog>
		</div>
	);
}

