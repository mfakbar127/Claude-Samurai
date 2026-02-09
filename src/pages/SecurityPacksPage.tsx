import { PackageIcon } from "lucide-react";
import { useMemo, useState } from "react";
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
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { ScrollArea } from "@/components/ui/scroll-area";
import { json } from "@codemirror/lang-json";
import CodeMirror from "@uiw/react-codemirror";
import { useCodeMirrorTheme } from "@/lib/use-codemirror-theme";
import { toast } from "sonner";
import {
	type InstalledSecurityPackItem,
	type McpTemplate,
	type MarketplaceTemplate,
	type KnownMarketplaces,
	type SecurityPackType,
	type SecurityTemplates,
	useInstallSecurityTemplate,
	useInstalledSecurityTemplates,
	useSecurityTemplates,
	useUninstallSecurityTemplate,
	useKnownMarketplaces,
} from "@/lib/query";
import {
	codeMirrorBasicSetup,
	markdownExtensions,
} from "@/lib/codemirror-config";

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

async function copyToClipboard(text: string): Promise<void> {
	try {
		if (navigator?.clipboard?.writeText) {
			await navigator.clipboard.writeText(text);
			toast.success("Command copied to clipboard");
		}
	} catch {
		// ignore copy errors
	}
}

type TemplateItem =
	| (SecurityTemplates["agents"][number] & { type: "agent" })
	| (SecurityTemplates["skills"][number] & { type: "skill" })
	| (SecurityTemplates["commands"][number] & { type: "command" })
	| (McpTemplate & { type: "mcp" })
	| (MarketplaceTemplate & { type: "marketplace" });

interface DetailState {
	item: TemplateItem;
	installed: boolean;
	marketplaceKey?: string;
}

function getMarketplaceKey(
	marketplaces: KnownMarketplaces | undefined,
	link: string,
): string | undefined {
	if (!marketplaces) {
		return undefined;
	}

	try {
		const url = new URL(link);
		const segments = url.pathname.split("/").filter(Boolean);

		if (segments.length < 2) {
			return undefined;
		}

		const repoName = `${segments[0]}/${segments[1]}`;

		for (const [key, value] of Object.entries(marketplaces)) {
			if (value.source.repo === repoName) {
				return key;
			}
		}
	} catch {
		// ignore URL parse errors
	}

	return undefined;
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

function getItemTitle(item: TemplateItem): string {
	if ("title" in item && typeof item.title === "string") {
		return item.title;
	}

	if ("name" in item && typeof (item as MarketplaceTemplate).name === "string") {
		return (item as MarketplaceTemplate).name;
	}

	return item.id;
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
		case "marketplace":
			return "Marketplace";
	}
}

function typeBadgeLabel(t: (key: string) => string, type: SecurityPackType): string {
	switch (type) {
		case "agent":
			return "Agent";
		case "skill":
			return t("navigation.skills");
		case "command":
			return t("navigation.commands");
		case "mcp":
			return t("navigation.mcp");
		case "marketplace":
			return "Marketplace";
	}
}

function SecurityPacksSection(props: {
	title: string;
	items: TemplateItem[];
	installedMap: Map<string, InstalledSecurityPackItem>;
	marketplaceInstalledMap?: Map<string, boolean>;
	onShowDetails: (item: TemplateItem, installed: boolean) => void;
	onToggleInstall: (item: TemplateItem, installed: boolean) => void;
}) {
	const { t } = useTranslation();
	if (props.items.length === 0) {
		return null;
	}

	return (
		<section className="space-y-3">
			<h3 className="text-sm font-semibold tracking-tight text-muted-foreground">
				{props.title}
			</h3>
			<Accordion type="multiple" className="space-y-2">
				{props.items.map((item) => {
					const itemKey = `${item.type}-${item.id}`;
					const installed =
						item.type === "marketplace"
							? props.marketplaceInstalledMap?.get(item.id) ?? false
							: props.installedMap.has(`${item.type}:${item.id}`);

					return (
						<AccordionItem
							key={itemKey}
							value={itemKey}
							className="bg-card border rounded-lg"
						>
							<AccordionTrigger className="hover:no-underline px-4 py-2 bg-card hover:bg-accent duration-150">
								<div className="flex items-center justify-between gap-2 w-full">
									<div className="flex items-center gap-2 flex-wrap">
										<PackageIcon size={12} />
										<span className="font-medium">
											{getItemTitle(item)}
										</span>
										<Badge
											variant={installed ? "success" : "outline"}
										>
											{installed ? "Installed" : "Not installed"}
										</Badge>
									</div>
									<div className="flex items-center gap-2">
										<Badge variant="secondary">
											{typeBadgeLabel(t, item.type)}
										</Badge>
									</div>
								</div>
							</AccordionTrigger>
							<AccordionContent className="pb-3 px-4">
								<p className="text-sm text-muted-foreground mb-3">
									{item.description}
								</p>
								{item.type === "marketplace" && "link" in item && (
									<p className="text-xs mb-3">
										<Button
											variant="link"
											size="sm"
											className="h-auto p-0 text-xs text-primary underline underline-offset-2"
											onClick={(event) => {
												event.stopPropagation();
												window.open(
													(item as MarketplaceTemplate).link,
													"_blank",
												);
											}}
										>
											More info
										</Button>
									</p>
								)}
								<div className="flex gap-2">
									<Button
										size="sm"
										variant="outline"
										onClick={(e) => {
											e.stopPropagation();
											props.onShowDetails(item, installed);
										}}
									>
										{item.type === "mcp"
											? "View config"
											: item.type === "marketplace"
												? "View instructions"
												: "View markdown"}
									</Button>
									<Button
										size="sm"
										variant={installed ? "outline" : "default"}
										onClick={(e) => {
											e.stopPropagation();
											props.onToggleInstall(item, installed);
										}}
									>
										{installed ? "Uninstall" : "Install"}
									</Button>
								</div>
							</AccordionContent>
						</AccordionItem>
					);
				})}
			</Accordion>
		</section>
	);
}

export function SecurityPacksPage() {
	const { t } = useTranslation();
	const { data: templates } = useSecurityTemplates();
	const { data: installedItems } = useInstalledSecurityTemplates();
	const { data: knownMarketplaces } = useKnownMarketplaces();
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

	const marketplaceInstalledMap = useMemo(() => {
		const map = new Map<string, boolean>();
		const marketplaces: KnownMarketplaces | undefined = knownMarketplaces;

		if (!templates?.marketplace || !marketplaces) {
			return map;
		}

		const installedRepos = new Set<string>();
		for (const value of Object.values(marketplaces)) {
			installedRepos.add(value.source.repo);
		}

		for (const item of templates.marketplace) {
			const marketplaceKey = getMarketplaceKey(marketplaces, item.link);

			if (!marketplaceKey) {
				map.set(item.id, false);
				continue;
			}

			const repoName = marketplaces[marketplaceKey]?.source.repo;
			map.set(item.id, repoName ? installedRepos.has(repoName) : false);
		}

		return map;
	}, [knownMarketplaces, templates?.marketplace]);

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
	const marketplaceItems: TemplateItem[] =
		templates?.marketplace.map((m) => ({ ...m, type: "marketplace" as const })) ??
		[];

	const itemsByType: Record<SecurityPackType, TemplateItem[]> = {
		agent: agentItems,
		skill: skillItems,
		command: commandItems,
		mcp: mcpItems,
		marketplace: marketplaceItems,
	};

	const selectedItems: TemplateItem[] = itemsByType[selectedType];

	function handleInstallUninstall(
		item: TemplateItem,
		installed: boolean,
		closeDialog?: boolean,
	): void {
		if (item.type === "marketplace") {
			// For marketplace entries, we only show CLI instructions in the detail dialog.
			const marketplaceKey = getMarketplaceKey(knownMarketplaces, item.link);

			setDetail({ item, installed, marketplaceKey });
			return;
		}
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
		if (current.item.type === "marketplace") {
			const marketplace = current.item as MarketplaceTemplate & {
				type: "marketplace";
			};
			const displayName = marketplace.name;
			const installCmd = `claude plugin marketplace add ${displayName}`;
			const keyArg = current.marketplaceKey ?? displayName;
			const uninstallCmd = `claude plugin marketplace remove ${keyArg}`;

			return (
				<div className="space-y-3">
					<p className="text-xs text-muted-foreground">
						{marketplace.description}
					</p>
					<div className="space-y-2">
						<p className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide">
							Install
						</p>
						<div className="flex items-center gap-2">
							<div className="rounded-md bg-muted px-3 py-2 font-mono text-xs break-all flex-1 select-text">
								{installCmd}
							</div>
							<Button
								size="sm"
								variant="outline"
								onClick={() => void copyToClipboard(installCmd)}
							>
								Copy
							</Button>
						</div>
					</div>
					<div className="space-y-2">
						<p className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide">
							Uninstall
						</p>
						<div className="flex items-center gap-2">
							<div className="rounded-md bg-muted px-3 py-2 font-mono text-xs break-all flex-1 select-text">
								{uninstallCmd}
							</div>
							<Button
								size="sm"
								variant="outline"
								onClick={() => void copyToClipboard(uninstallCmd)}
							>
								Copy
							</Button>
						</div>
					</div>
				</div>
			);
		}

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
				<div className="rounded-lg overflow-hidden border">
					<CodeMirror
						value={markdown}
						height="260px"
						theme={codeMirrorTheme}
						extensions={markdownExtensions}
						basicSetup={codeMirrorBasicSetup}
						readOnly
					/>
				</div>
			</div>
		);
	}

	function handleDetailPrimary(current: DetailState): void {
		if (current.item.type === "marketplace") {
			setDetail(null);
			return;
		}
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
				<div className="flex gap-1 border border-border rounded-md p-1">
					<Button
						size="sm"
						variant={selectedType === "agent" ? "default" : "ghost"}
						className="text-xs"
						data-active={selectedType === "agent"}
						aria-current={selectedType === "agent" ? "true" : undefined}
						onClick={() => setSelectedType("agent")}
					>
						Agents
					</Button>
					<Button
						size="sm"
						variant={selectedType === "skill" ? "default" : "ghost"}
						className="text-xs"
						data-active={selectedType === "skill"}
						aria-current={selectedType === "skill" ? "true" : undefined}
						onClick={() => setSelectedType("skill")}
					>
						{t("navigation.skills")}
					</Button>
					<Button
						size="sm"
						variant={selectedType === "command" ? "default" : "ghost"}
						className="text-xs"
						data-active={selectedType === "command"}
						aria-current={selectedType === "command" ? "true" : undefined}
						onClick={() => setSelectedType("command")}
					>
						{t("navigation.commands")}
					</Button>
					<Button
						size="sm"
						variant={selectedType === "mcp" ? "default" : "ghost"}
						className="text-xs"
						data-active={selectedType === "mcp"}
						aria-current={selectedType === "mcp" ? "true" : undefined}
						onClick={() => setSelectedType("mcp")}
					>
						{t("navigation.mcp")}
					</Button>
					<Button
						size="sm"
						variant={selectedType === "marketplace" ? "default" : "ghost"}
						className="text-xs"
						data-active={selectedType === "marketplace"}
						aria-current={selectedType === "marketplace" ? "true" : undefined}
						onClick={() => setSelectedType("marketplace")}
					>
						Marketplace
					</Button>
				</div>
			</div>
			<ScrollArea className="flex-1 h-full">
				<div className="p-3 space-y-6">
					<SecurityPacksSection
						title={sectionLabel(t, selectedType)}
						items={selectedItems}
						installedMap={installedMap}
						marketplaceInstalledMap={marketplaceInstalledMap}
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
									<span>{getItemTitle(detail.item)}</span>
									<Badge
										variant={
											detail.installed ? "success" : "outline"
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

