import { message } from "@tauri-apps/plugin-dialog";
import { BotIcon, PackageIcon, TerminalIcon } from "lucide-react";
import { Suspense } from "react";
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
import { useInstalledPlugins, useTogglePlugin } from "@/lib/query";

function PluginsPageContent() {
	const { t } = useTranslation();
	const { data: plugins, isLoading, error } = useInstalledPlugins();
	const togglePlugin = useTogglePlugin();

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
					{t("plugins.error", { error: error.message })}
				</div>
			</div>
		);
	}

	const sortedPlugins = [...(plugins ?? [])].sort((a, b) => {
		const aDate = a.installedAt ?? "";
		const bDate = b.installedAt ?? "";
		return bDate.localeCompare(aDate);
	});

	const handleTogglePlugin = async (
		pluginName: string,
		currentEnabled: boolean,
		scope: string,
		projectPath?: string,
	) => {
		if (scope === "local" && !projectPath) {
			await message(t("plugins.toggleLocalError", { pluginName }), {
				title: t("plugins.validationError"),
				kind: "error",
			});
			return;
		}

		togglePlugin.mutate({
			pluginName,
			enabled: !currentEnabled,
			scope,
			projectPath,
		});
	};

	return (
		<div>
			<div
				className="flex items-center justify-between sticky top-0 z-10 border-b p-3 bg-background"
				data-tauri-drag-region
			>
				<div data-tauri-drag-region>
					<h3 className="font-bold" data-tauri-drag-region>
						{t("plugins.title")}
					</h3>
					<p className="text-sm text-muted-foreground" data-tauri-drag-region>
						{t("plugins.description")}
					</p>
				</div>
			</div>
			<div>
				{!plugins || plugins.length === 0 ? (
					<div className="text-center text-muted-foreground py-8">
						{t("plugins.noPlugins")}
					</div>
				) : (
					<ScrollArea className="h-full">
						<div>
							<Accordion type="multiple">
								{sortedPlugins.map((plugin) => (
									<AccordionItem
										key={`${plugin.name}-${plugin.scope}`}
										value={`${plugin.name}-${plugin.scope}`}
										className="bg-card"
									>
										<AccordionTrigger className="hover:no-underline px-4 py-2 bg-card hover:bg-accent duration-150">
											<div className="flex flex-col items-start gap-1 w-full">
												<div className="flex items-center gap-2 flex-wrap">
													<PackageIcon size={12} />
													<span className="font-medium">{plugin.name}</span>
													<Badge
														variant={
															plugin.scope === "user" ? "default" : "secondary"
														}
													>
														{plugin.scope === "user"
															? t("plugins.scopeUser")
															: t("plugins.scopeLocal")}
													</Badge>
													<Badge
														variant={plugin.enabled ? "success" : "outline"}
													>
														{plugin.enabled
															? t("plugins.enabled")
															: t("plugins.disabled")}
													</Badge>
													{plugin.scope === "local" && plugin.projectPath ? (
														<span className="text-xs text-muted-foreground font-mono">
															{plugin.projectPath}
														</span>
													) : (
														<span className="text-sm text-muted-foreground font-normal">
															v{plugin.version}
														</span>
													)}
												</div>
											</div>
										</AccordionTrigger>
										<AccordionContent className="pb-3">
											<div className="px-3 pt-3 space-y-3">
												<div className="flex flex-wrap gap-2">
													<span className="text-sm text-muted-foreground">
														{t("plugins.packages")}:
													</span>
													{plugin.packages.hasAgents && (
														<Badge variant="secondary">
															<BotIcon size={12} className="mr-1" />
															{t("plugins.packageAgents")}
														</Badge>
													)}
													{plugin.packages.hasSkills && (
														<Badge variant="secondary">
															{t("plugins.packageSkills")}
														</Badge>
													)}
													{plugin.packages.hasCommands && (
														<Badge variant="secondary">
															<TerminalIcon size={12} className="mr-1" />
															{t("plugins.packageCommands")}
														</Badge>
													)}
													{plugin.packages.hasMcp && (
														<Badge variant="secondary">
															{t("plugins.packageMcp")}
														</Badge>
													)}
													{!plugin.packages.hasAgents &&
														!plugin.packages.hasSkills &&
														!plugin.packages.hasCommands &&
														!plugin.packages.hasMcp && (
															<span className="text-sm text-muted-foreground">
																{t("plugins.none")}
															</span>
														)}
												</div>

												<div className="flex justify-between bg-card">
													<Button
														variant={plugin.enabled ? "outline" : "default"}
														size="sm"
														onClick={() =>
															handleTogglePlugin(
																plugin.name,
																plugin.enabled,
																plugin.scope,
																plugin.projectPath,
															)
														}
														disabled={togglePlugin.isPending}
													>
														{plugin.enabled
															? t("plugins.disable")
															: t("plugins.enable")}
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

export function PluginsPage() {
	const { t } = useTranslation();

	return (
		<Suspense
			fallback={
				<div className="flex items-center justify-center min-h-screen">
					<div className="text-center">{t("loading")}</div>
				</div>
			}
		>
			<PluginsPageContent />
		</Suspense>
	);
}
