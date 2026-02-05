import { ask, message } from "@tauri-apps/plugin-dialog";
import CodeMirror from "@uiw/react-codemirror";
import { PlusIcon, SaveIcon, TerminalIcon, TrashIcon } from "lucide-react";
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
	useClaudeCommands,
	useDeleteClaudeCommand,
	usePluginCommands,
	useToggleClaudeCommand,
	useWriteClaudeCommand,
} from "@/lib/query";
import { useCodeMirrorTheme } from "@/lib/use-codemirror-theme";

type UnifiedCommand = {
	name: string;
	content: string;
	exists: boolean;
	disabled: boolean;
	source: "user" | "plugin";
	pluginName?: string;
	pluginScope?: string;
	sourcePath: string;
};

function CommandsPageContent() {
	const { t } = useTranslation();
	const { data: userCommands, isLoading: isLoadingUser, error: errorUser } = useClaudeCommands();
	const { data: pluginCommands, isLoading: isLoadingPlugin, error: errorPlugin } = usePluginCommands();
	const writeCommand = useWriteClaudeCommand();
	const deleteCommand = useDeleteClaudeCommand();
	const toggleCommand = useToggleClaudeCommand();
	const [commandEdits, setCommandEdits] = useState<Record<string, string>>({});
	const [isDialogOpen, setIsDialogOpen] = useState(false);
	const codeMirrorTheme = useCodeMirrorTheme();

	const isLoading = isLoadingUser || isLoadingPlugin;
	const error = errorUser || errorPlugin;

	const commands: UnifiedCommand[] = [
		...(userCommands || []).map((cmd): UnifiedCommand => ({
			name: cmd.name,
			content: cmd.content,
			exists: cmd.exists,
			disabled: cmd.disabled,
			source: "user",
			sourcePath: `~/.claude/commands/${cmd.name}.md${cmd.disabled ? '.disabled' : ''}`,
		})),
		...(pluginCommands || []).map((cmd): UnifiedCommand => ({
			name: cmd.name,
			content: cmd.content,
			exists: cmd.exists,
			disabled: cmd.disabled,
			source: "plugin",
			pluginName: cmd.pluginName,
			pluginScope: cmd.pluginScope,
			sourcePath: cmd.sourcePath,
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
					{t("commands.error", { error: error.message })}
				</div>
			</div>
		);
	}

	const handleContentChange = (commandName: string, content: string) => {
		setCommandEdits((prev) => ({
			...prev,
			[commandName]: content,
		}));
	};

	const handleSaveCommand = async (commandName: string) => {
		const content = commandEdits[commandName];
		if (content === undefined) return;

		writeCommand.mutate({
			commandName,
			content,
		});
	};

	const handleDeleteCommand = async (commandName: string) => {
		const confirmed = await ask(t("commands.deleteConfirm", { commandName }), {
			title: t("commands.deleteTitle"),
			kind: "warning",
		});

		if (confirmed) {
			deleteCommand.mutate(commandName);
		}
	};

	const handleToggleCommand = (commandName: string, currentDisabled: boolean) => {
		toggleCommand.mutate({ commandName, disabled: !currentDisabled });
	};

	return (
		<div>
			<div
				className="flex items-center justify-between sticky top-0 z-10 border-b p-3 bg-background"
				data-tauri-drag-region
			>
				<div data-tauri-drag-region>
					<h3 className="font-bold" data-tauri-drag-region>
						{t("commands.title")}
					</h3>
					<p className="text-sm text-muted-foreground" data-tauri-drag-region>
						{t("commands.description")}
					</p>
				</div>
				<Dialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
					<DialogTrigger asChild>
						<Button variant="ghost" className="text-muted-foreground" size="sm">
							<PlusIcon size={14} />
							{t("commands.addCommand")}
						</Button>
					</DialogTrigger>
					<DialogContent className="max-w-[600px]">
						<DialogHeader>
							<DialogTitle>{t("commands.addCommandTitle")}</DialogTitle>
							<DialogDescription className="text-muted-foreground text-sm">
								{t("commands.addCommandDescription")}
							</DialogDescription>
						</DialogHeader>
						<CreateCommandPanel onClose={() => setIsDialogOpen(false)} />
					</DialogContent>
				</Dialog>
			</div>
			<div>
				{commands.length === 0 ? (
					<div className="text-center text-muted-foreground py-8">
						{t("commands.noCommands")}
					</div>
				) : (
					<ScrollArea className="h-full">
						<div>
							<Accordion type="multiple">
								{commands.map((command) => (
									<AccordionItem
										key={command.name}
										value={command.name}
										className="bg-card"
									>
										<AccordionTrigger className="hover:no-underline px-4 py-2 bg-card hover:bg-accent duration-150">
											<div className="flex items-center justify-between gap-2 w-full">
												<div className="flex items-center gap-2 flex-wrap">
													<TerminalIcon size={12} />
													<span className="font-medium">{command.name}</span>
													<Badge
														variant={command.disabled ? "secondary" : "success"}
													>
														{command.disabled
															? t("commands.disabled")
															: t("commands.enabled")}
													</Badge>
													<span className="text-xs text-muted-foreground font-mono truncate max-w-xs">
														{command.sourcePath}
													</span>
												</div>
												<div className="flex items-center gap-2">
													{command.source === "user" ? (
														<Badge variant="default">
															{t("commands.sourceUser")}
														</Badge>
													) : (
														<>
															<Badge variant="outline">
																{command.pluginName}
															</Badge>
															<Badge
																variant={
																	command.pluginScope === "user"
																		? "default"
																		: "secondary"
																}
															>
																{command.pluginScope === "user"
																	? t("plugins.scopeUser")
																	: t("plugins.scopeLocal")}
															</Badge>
														</>
													)}
												</div>
											</div>
										</AccordionTrigger>
										<AccordionContent className="pb-3">
											<div className="px-3 pt-3 space-y-3">
												<div className="rounded-lg overflow-hidden border">
													<CodeMirror
														value={commandEdits[command.name] ?? command.content}
														height="180px"
														theme={codeMirrorTheme}
														onChange={(value) =>
															handleContentChange(command.name, value)
														}
														placeholder={t("commands.contentPlaceholder")}
														extensions={markdownExtensions}
														basicSetup={codeMirrorBasicSetup}
													/>
												</div>
												<div className="flex justify-between bg-card px-1 py-1">
													<div className="flex items-center text-xs text-muted-foreground font-mono">
														<span className="truncate max-w-xs">
															{command.sourcePath}
														</span>
													</div>
													<div className="flex gap-2">
														<Button
															variant="outline"
															size="sm"
															onClick={() => handleSaveCommand(command.name)}
															disabled={
																writeCommand.isPending ||
																commandEdits[command.name] === undefined
															}
														>
															<SaveIcon size={12} className="mr-1" />
															{writeCommand.isPending
																? t("commands.saving")
																: t("commands.save")}
														</Button>
														<Button
															variant="outline"
															size="sm"
															onClick={() =>
																handleToggleCommand(command.name, command.disabled)
															}
															disabled={toggleCommand.isPending}
														>
															{command.disabled
																? t("commands.enable")
																: t("commands.disable")}
														</Button>
														<Button
															variant="outline"
															size="sm"
															onClick={() => handleDeleteCommand(command.name)}
															disabled={deleteCommand.isPending}
														>
															<TrashIcon size={12} className="mr-1" />
															Delete
														</Button>
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

export function CommandsPage() {
	const { t } = useTranslation();

	return (
		<Suspense
			fallback={
				<div className="flex items-center justify-center min-h-screen">
					<div className="text-center">{t("loading")}</div>
				</div>
			}
		>
			<CommandsPageContent />
		</Suspense>
	);
}

type CreateCommandPanelProps = {
	onClose?: () => void;
};

function CreateCommandPanel({ onClose }: CreateCommandPanelProps) {
	const { t } = useTranslation();
	const [commandName, setCommandName] = useState("");
	const [commandContent, setCommandContent] = useState("");
	const writeCommand = useWriteClaudeCommand();
	const { data: commands } = useClaudeCommands();
	const codeMirrorTheme = useCodeMirrorTheme();

	const handleCreateCommand = async () => {
		if (!commandName.trim()) {
			await message(t("commands.emptyNameError"), {
				title: t("commands.validationError"),
				kind: "error",
			});
			return;
		}

		if (commands?.some((cmd) => cmd.name === commandName)) {
			await message(t("commands.commandExistsError", { commandName }), {
				title: t("commands.commandExistsTitle"),
				kind: "info",
			});
			return;
		}

		if (!commandContent.trim()) {
			await message(t("commands.emptyContentError"), {
				title: t("commands.validationError"),
				kind: "error",
			});
			return;
		}

		writeCommand.mutate(
			{
				commandName,
				content: commandContent,
			},
			{
				onSuccess: () => {
					setCommandName("");
					setCommandContent("");
					onClose?.();
				},
			},
		);
	};

	return (
		<div className="space-y-4 mt-4">
			<div className="space-y-2">
				<Label htmlFor="command-name">{t("commands.commandName")}</Label>
				<Input
					id="command-name"
					value={commandName}
					onChange={(e) => setCommandName(e.target.value)}
					placeholder={t("commands.commandNamePlaceholder")}
				/>
			</div>

			<div className="space-y-2">
				<Label htmlFor="command-content">{t("commands.commandContent")}</Label>
				<div className="rounded-lg overflow-hidden border">
					<CodeMirror
						value={commandContent}
						height="200px"
						theme={codeMirrorTheme}
						onChange={(value) => setCommandContent(value)}
						placeholder={t("commands.contentPlaceholder")}
						extensions={markdownExtensions}
						basicSetup={codeMirrorBasicSetup}
					/>
				</div>
			</div>

			<div className="flex justify-end">
				<Button
					onClick={handleCreateCommand}
					disabled={
						!commandName.trim() ||
						!commandContent.trim() ||
						writeCommand.isPending
					}
				>
					{writeCommand.isPending
						? t("commands.creating")
						: t("commands.create")}
				</Button>
			</div>
		</div>
	);
}
