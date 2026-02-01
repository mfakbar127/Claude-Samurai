import { json } from "@codemirror/lang-json";
import { ask, message } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import CodeMirror from "@uiw/react-codemirror";
import {
	ExternalLinkIcon,
	HammerIcon,
	PlusIcon,
	SaveIcon,
	TrashIcon,
} from "lucide-react";
import { Suspense, useState } from "react";
import { useTranslation } from "react-i18next";
import { match } from "ts-pattern";
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
import { builtInMcpServers } from "@/lib/builtInMCP";
import {
	type McpServerState,
	useAddGlobalMcpServer,
	useDeleteGlobalMcpServer,
	useGetMcpServersWithState,
	useToggleMcpServer,
	useUpdateGlobalMcpServer,
} from "@/lib/query";
import { useCodeMirrorTheme } from "@/lib/use-codemirror-theme";

function MCPPageContent() {
	const { t } = useTranslation();
	const { data: mcpServersWithState } = useGetMcpServersWithState();
	const updateMcpServer = useUpdateGlobalMcpServer();
	const deleteMcpServer = useDeleteGlobalMcpServer();
	const toggleMcpServer = useToggleMcpServer();
	const [serverConfigs, setServerConfigs] = useState<Record<string, string>>(
		{},
	);
	const [isDialogOpen, setIsDialogOpen] = useState(false);
	const codeMirrorTheme = useCodeMirrorTheme();

	// Helper functions for badge display
	const getBadgeVariant = (state: string) => {
		switch (state) {
			case "enabled":
				return "success";
			case "disabled":
				return "outline";
			case "runtime-disabled":
				return "secondary";
			default:
				return "outline";
		}
	};

	const getBadgeLabel = (state: string) => {
		switch (state) {
			case "enabled":
				return t("mcp.enabled");
			case "disabled":
				return t("mcp.disabled");
			case "runtime-disabled":
				return t("mcp.runtimeDisabled");
			default:
				return t("mcp.disabled");
		}
	};

	const handleToggleServer = async (server: McpServerState) => {
		if (!server.controllable) return;
		
		const currentlyEnabled = server.state === "enabled" || server.state === "runtime-disabled";
		toggleMcpServer.mutate({
			serverName: server.name,
			enabled: !currentlyEnabled,
		});
	};

	const handleConfigChange = (serverName: string, configText: string) => {
		setServerConfigs((prev) => ({
			...prev,
			[serverName]: configText,
		}));
	};

	const handleSaveConfig = async (serverName: string) => {
		const configText = serverConfigs[serverName];
		if (!configText) return;

		try {
			const configObject = JSON.parse(configText);
			updateMcpServer.mutate({
				serverName,
				serverConfig: configObject,
			});
		} catch (error) {
			await message(t("mcp.invalidJsonError", { serverName }), {
				title: t("mcp.invalidJsonTitle"),
				kind: "error",
			});
		}
	};

	const handleDeleteServer = async (serverName: string) => {
		// Show confirmation dialog
		const confirmed = await ask(t("mcp.deleteServerConfirm", { serverName }), {
			title: t("mcp.deleteServerTitle"),
			kind: "warning",
		});

		if (confirmed) {
			deleteMcpServer.mutate(serverName);
		}
	};

	const formatConfigForDisplay = (server: McpServerState): string => {
		return JSON.stringify(server.config, null, 2);
	};

	const sortedServers = [...mcpServersWithState].sort((a, b) =>
		a.name.localeCompare(b.name),
	);

	return (
		<div className="">
			<div
				className="flex items-center p-3 border-b px-3 justify-between sticky top-0 bg-background z-10"
				data-tauri-drag-region
			>
				<div data-tauri-drag-region>
					<h3 className="font-bold" data-tauri-drag-region>
						{t("mcp.title")}
					</h3>
					<p className="text-sm text-muted-foreground" data-tauri-drag-region>
						{t("mcp.description")}
					</p>
				</div>
				<Dialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
					<DialogTrigger asChild>
						<Button variant="ghost" className="text-muted-foreground" size="sm">
							<PlusIcon size={14} />
							{t("mcp.addServer")}
						</Button>
					</DialogTrigger>
					<DialogContent className="max-w-[700px] h-[500px]">
						<DialogHeader>
							<DialogTitle className="text-primary text-sm">
								{t("mcp.addServerTitle")}
							</DialogTitle>
							<DialogDescription className="text-muted-foreground text-sm">
								{t("mcp.addServerDescription")}
							</DialogDescription>
						</DialogHeader>
						<div className="py-3 mt-3">
							<MCPCreatePanel onClose={() => setIsDialogOpen(false)} />
						</div>
						{/* <div className="flex justify-end">
              <Button
                variant="outline"
                onClick={() => setIsDialogOpen(false)}
              >
                关闭
              </Button>
            </div> */}
					</DialogContent>
				</Dialog>
			</div>
			<div className="">
				{sortedServers.length === 0 ? (
					<div className="text-center text-muted-foreground py-8">
						{t("mcp.noServersConfigured")}
					</div>
				) : (
					<Accordion type="multiple" className="">
						{sortedServers.map((server) => (
							<AccordionItem
								key={server.name}
								value={server.name}
								className="bg-card"
							>
								<AccordionTrigger className="hover:no-underline px-4 py-2 bg-card hover:bg-accent  duration-150">
									<div className="flex items-center gap-2">
										<HammerIcon size={12} />
										<span className="font-medium">{server.name}</span>
										<Badge variant={getBadgeVariant(server.state)}>
											{getBadgeLabel(server.state)}
										</Badge>
										<Badge variant="outline" className="text-xs">
											{server.sourceType}
										</Badge>
									</div>
								</AccordionTrigger>
								<AccordionContent className="pb-3">
									<div className="px-3 pt-3 space-y-3">
										<div className="rounded-lg overflow-hidden border">
											<CodeMirror
												value={
													serverConfigs[server.name] ||
													formatConfigForDisplay(server)
												}
												height="180px"
												theme={codeMirrorTheme}
												extensions={[json()]}
												onChange={(value) =>
													handleConfigChange(server.name, value)
												}
												placeholder="Enter MCP server configuration as JSON"
											/>
										</div>
										<div className="flex justify-between  bg-card">
											<div className="flex gap-2">
												<Button
													variant="outline"
													onClick={() => handleSaveConfig(server.name)}
													disabled={updateMcpServer.isPending}
													size="sm"
												>
													<SaveIcon size={14} className="" />
													{updateMcpServer.isPending
														? t("mcp.saving")
														: t("mcp.save")}
												</Button>

												<Button
													variant="ghost"
													size="sm"
													onClick={() => handleDeleteServer(server.name)}
													disabled={deleteMcpServer.isPending}
												>
													<TrashIcon size={14} className="" />
												</Button>
											</div>

											<Button
												variant={
													server.state === "enabled" ? "outline" : "default"
												}
												size="sm"
												onClick={() => handleToggleServer(server)}
												disabled={!server.controllable || toggleMcpServer.isPending}
												title={
													!server.controllable
														? t("mcp.notControllable")
														: undefined
												}
											>
												{server.state === "enabled"
													? t("mcp.disable")
													: t("mcp.enable")}
											</Button>
										</div>
									</div>
								</AccordionContent>
							</AccordionItem>
						))}
					</Accordion>
				)}
			</div>
		</div>
	);
}

export function MCPPage() {
	return (
		<Suspense
			fallback={
				<div className="flex items-center justify-center min-h-screen">
					<div className="text-center">Loading MCP servers...</div>
				</div>
			}
		>
			<MCPPageContent />
		</Suspense>
	);
}

function MCPCreatePanel({ onClose }: { onClose?: () => void }) {
	const { t } = useTranslation();
	const [currentTab, setCurrentTab] = useState<"recommend" | "manual">(
		"recommend",
	);

	return (
		<div className="">
			<div className="flex mb-3 gap-1">
				<Button
					size="sm"
					variant={currentTab === "recommend" ? "secondary" : "ghost"}
					className="text-sm"
					onClick={() => setCurrentTab("recommend")}
				>
					{t("mcp.recommend")}
				</Button>
				<Button
					size="sm"
					variant={currentTab === "manual" ? "secondary" : "ghost"}
					className="text-sm"
					onClick={() => setCurrentTab("manual")}
				>
					{t("mcp.custom")}
				</Button>
			</div>

			{match(currentTab)
				.with("recommend", () => {
					return <RecommendMCPPanel onClose={onClose} />;
				})
				.with("manual", () => {
					return <CustomMCPPanel onClose={onClose} />;
				})
				.exhaustive()}
		</div>
	);
}

function RecommendMCPPanel({ onClose }: { onClose?: () => void }) {
	const { t } = useTranslation();
	const addMcpServer = useAddGlobalMcpServer();
	const { data: mcpServersWithState } = useGetMcpServersWithState();

	const handleAddMcpServer = async (
		mcpServer: (typeof builtInMcpServers)[0],
	) => {
		try {
			// Check if MCP server already exists using cached data
			const exists =
				mcpServersWithState && mcpServersWithState.some(s => s.name === mcpServer.name);

			if (exists) {
				await message(
					t("mcp.serverExistsError", { serverName: mcpServer.name }),
					{
						title: t("mcp.serverExistsTitle"),
						kind: "info",
					},
				);
				return;
			}

			// Show confirmation dialog
			const confirmed = await ask(
				t("mcp.addServerConfirm", { serverName: mcpServer.name }),
				{ title: t("mcp.addServerTitle"), kind: "info" },
			);

			if (confirmed) {
				// Parse the prefill JSON to get the config object
				const configObject = JSON.parse(`{${mcpServer.prefill}}`);

				addMcpServer.mutate(
					{
						serverName: mcpServer.name,
						serverConfig: configObject[mcpServer.name],
					},
					{
						onSuccess: () => {
							// Close dialog after successful addition
							onClose?.();
						},
					},
				);
			}
		} catch (error) {
			console.error("Failed to add MCP server:", error);
			await message(t("mcp.addServerError"), {
				title: "Error",
				kind: "error",
			});
		}
	};

	return (
		<div className="grid grid-cols-3 gap-5">
			{builtInMcpServers.map((mcpServer) => (
				<div
					key={mcpServer.name}
					className="border p-3 rounded-md h-[120px] flex justify-between flex-col hover:bg-primary/10 hover:border-primary/20 hover:text-primary cursor-default"
					onClick={() => handleAddMcpServer(mcpServer)}
				>
					<div className="flex justify-between items-center">
						<h3 className="font-bold text-primary">{mcpServer.name}</h3>
						<a
							onClick={(e) => {
								e.stopPropagation();
								openUrl(mcpServer.source);
							}}
							className="text-sm text-muted-foreground flex items-center gap-1 hover:underline"
						>
							<ExternalLinkIcon size={12} />
							{t("mcp.source")}
						</a>
					</div>
					<div></div>
					<div className="space-y-3">
						<p className="text-sm text-muted-foreground">
							{mcpServer.description}
						</p>
						{/* <Button size="sm" variant="outline" className="w-full text-sm">
          <PlusIcon />
          添加
        </Button> */}
					</div>
				</div>
			))}
		</div>
	);
}

function CustomMCPPanel({ onClose }: { onClose?: () => void }) {
	const { t } = useTranslation();
	const [customConfig, setCustomConfig] = useState("");
	const addMcpServer = useAddGlobalMcpServer();
	const { data: mcpServersWithState } = useGetMcpServersWithState();
	const codeMirrorTheme = useCodeMirrorTheme();

	const handleAddCustomMcpServer = async () => {
		try {
			// Validate JSON format
			let configObject;
			try {
				configObject = JSON.parse(customConfig);
			} catch (error) {
				await message(t("mcp.addCustomServerError"), {
					title: t("mcp.invalidJsonTitle"),
					kind: "error",
				});
				return;
			}

			// Check if it's an object with at least one server
			if (typeof configObject !== "object" || configObject === null) {
				await message(t("mcp.invalidConfigError"), {
					title: "Invalid Configuration",
					kind: "error",
				});
				return;
			}

			const serverNames = Object.keys(configObject);
			if (serverNames.length === 0) {
				await message(t("mcp.noServersError"), {
					title: "Invalid Configuration",
					kind: "error",
				});
				return;
			}

			// Check for duplicate server names
			const existingNames = mcpServersWithState ? mcpServersWithState.map(s => s.name) : [];
			const duplicateNames = serverNames.filter((name) =>
				existingNames.includes(name),
			);

			if (duplicateNames.length > 0) {
				await message(
					t("mcp.duplicateServersError", {
						servers: duplicateNames.join(", "),
					}),
					{
						title: t("mcp.duplicateServersTitle"),
						kind: "warning",
					},
				);
				return;
			}

			// Show confirmation dialog
			const confirmed = await ask(
				t("mcp.addCustomServersConfirm", { count: serverNames.length }),
				{ title: t("mcp.addCustomServersTitle"), kind: "info" },
			);

			if (confirmed) {
				// Add each server
				for (const [serverName, serverConfig] of Object.entries(configObject)) {
					addMcpServer.mutate({
						serverName,
						serverConfig: serverConfig as Record<string, any>,
					});
				}

				// Clear input and close dialog
				setCustomConfig("");
				onClose?.();
			}
		} catch (error) {
			console.error("Failed to add custom MCP servers:", error);
			await message(t("mcp.addServerError"), {
				title: "Error",
				kind: "error",
			});
		}
	};

	return (
		<div className="">
			<div className="space-y-3">
				<div className="rounded-lg overflow-hidden border">
					<CodeMirror
						value={customConfig}
						onChange={(value) => setCustomConfig(value)}
						height="240px"
						theme={codeMirrorTheme}
						extensions={[json()]}
						placeholder={t("mcp.customPlaceholder")}
					/>
				</div>

				<div>
					<Button
						size="sm"
						variant="outline"
						className="w-full text-sm"
						onClick={handleAddCustomMcpServer}
						disabled={!customConfig.trim()}
					>
						{t("mcp.add")}
					</Button>
				</div>
			</div>
		</div>
	);
}
