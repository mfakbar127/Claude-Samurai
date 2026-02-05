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
import { codeMirrorBasicSetup, markdownExtensions } from "@/lib/codemirror-config";
import {
	type SkillFile,
	useClaudeSkills,
	useDeleteClaudeSkill,
	useToggleClaudeSkill,
	useWriteClaudeSkill,
} from "@/lib/query";
import { useCodeMirrorTheme } from "@/lib/use-codemirror-theme";

function SkillsPageContent() {
	const { t } = useTranslation();
	const { data: skills, isLoading, error } = useClaudeSkills();
	const toggleSkill = useToggleClaudeSkill();
	const writeSkill = useWriteClaudeSkill();
	const deleteSkill = useDeleteClaudeSkill();
	const [skillEdits, setSkillEdits] = useState<Record<string, string>>({});
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
					{t("skills.error", { error: error.message })}
				</div>
			</div>
		);
	}

	const sourceOrder: Record<"global" | "plugin" | "project", number> = {
		global: 0,
		plugin: 1,
		project: 2,
	};

	const skillList = (skills ?? []).slice().sort((a, b) => {
		const orderA = sourceOrder[a.source] ?? 99;
		const orderB = sourceOrder[b.source] ?? 99;

		if (orderA !== orderB) {
			return orderA - orderB;
		}

		return a.name.localeCompare(b.name);
	});

	const handleSaveSkill = (skill: SkillFile) => {
		if (skill.source === "plugin") {
			return;
		}

		const content = skillEdits[skill.name] ?? skill.content;

		writeSkill.mutate({
			name: skill.name,
			source: skill.source,
			projectPath: skill.projectPath,
			content,
			disabled: skill.disabled,
		});
	};

	const handleToggleSkill = (skill: SkillFile) => {
		if (skill.source === "plugin") {
			return;
		}

		toggleSkill.mutate({
			name: skill.name,
			source: skill.source,
			projectPath: skill.projectPath,
			disabled: !skill.disabled,
		});
	};

	const handleDeleteSkill = async (skill: SkillFile) => {
		if (skill.source === "plugin") {
			return;
		}

		const confirmed = await ask(`Delete skill ${skill.name}?`, {
			title: "Delete skill",
			kind: "warning",
		});

		if (!confirmed) {
			return;
		}

		deleteSkill.mutate({
			name: skill.name,
			source: skill.source,
			projectPath: skill.projectPath,
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
						{t("skills.title")}
					</h3>
					<p className="text-sm text-muted-foreground" data-tauri-drag-region>
						{t("skills.description")}
					</p>
				</div>
			</div>
			<div>
				{skillList.length === 0 ? (
					<div className="text-center text-muted-foreground py-8">
						{t("skills.noSkills")}
					</div>
				) : (
					<ScrollArea className="h-full">
						<div>
							<Accordion type="multiple">
								{skillList.map((skill) => (
									<AccordionItem
										key={skill.name}
										value={skill.name}
										className="bg-card"
									>
										<AccordionTrigger className="hover:no-underline px-4 py-2 bg-card hover:bg-accent duration-150">
											<div className="flex items-center justify-between gap-2 w-full">
												<div className="flex items-center gap-2 flex-wrap">
													<SparklesIcon size={12} />
													<span className="font-medium">{skill.name}</span>
													<Badge
														variant={skill.disabled ? "destructive" : "success"}
													>
														{skill.disabled ? "Disabled" : "Enabled"}
													</Badge>
													{skill.source === "project" && skill.projectPath && (
														<span className="text-xs text-muted-foreground font-mono truncate max-w-xs">
															{skill.projectPath}
														</span>
													)}
												</div>
												<div className="flex items-center gap-2">
													{skill.source === "global" && (
														<Badge variant="secondary">Global</Badge>
													)}
													{skill.source === "project" && (
														<Badge variant="secondary">Project</Badge>
													)}
													{skill.source === "plugin" && skill.pluginName && (
														<Badge variant="secondary">
															Plugins ({skill.pluginName})
														</Badge>
													)}
												</div>
											</div>
										</AccordionTrigger>
										<AccordionContent className="pb-3">
											<div className="px-3 pt-3 space-y-3">
												<div className="rounded-lg overflow-hidden border">
													<CodeMirror
														value={skillEdits[skill.name] ?? skill.content}
														height="280px"
														theme={codeMirrorTheme}
														onChange={(value) =>
															setSkillEdits((prev) => ({
																...prev,
																[skill.name]: value,
															}))
														}
														placeholder={t("skills.contentPlaceholder")}
														extensions={markdownExtensions}
														basicSetup={codeMirrorBasicSetup}
													/>
												</div>
												<div className="flex justify-between bg-card px-1 py-1">
													<div className="flex items-center text-xs text-muted-foreground font-mono">
														{skill.source === "global" && (
															<span>{`~/.claude/skills/${skill.name}/SKILL${
																skill.disabled ? ".md.disabled" : ".md"
															}`}</span>
														)}
														{skill.source === "project" && skill.projectPath && (
															<span>{`${skill.projectPath}/.claude/skills/${skill.name}/SKILL${
																skill.disabled ? ".md.disabled" : ".md"
															}`}</span>
														)}
														{skill.source === "plugin" && skill.pluginName && (
															<span>{`Plugin: ${skill.pluginName}`}</span>
														)}
													</div>
													<div className="flex gap-2">
														<Button
														variant="outline"
														size="sm"
															disabled={
																skill.source === "plugin" || writeSkill.isPending
															}
															onClick={() => handleSaveSkill(skill)}
														>
															<SaveIcon size={12} className="mr-1" />
															Save
														</Button>
														<Button
														variant="outline"
														size="sm"
															disabled={
																skill.source === "plugin" || toggleSkill.isPending
															}
															onClick={() => handleToggleSkill(skill)}
														>
															{skill.disabled ? "Enable" : "Disable"}
														</Button>
														<Button
														variant="outline"
														size="sm"
															disabled={
																skill.source === "plugin" || deleteSkill.isPending
															}
															onClick={() => {
																void handleDeleteSkill(skill);
															}}
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

export function SkillsPage() {
	const { t } = useTranslation();

	return (
		<Suspense
			fallback={
				<div className="flex items-center justify-center min-h-screen">
					<div className="text-center">{t("loading")}</div>
				</div>
			}
		>
			<SkillsPageContent />
		</Suspense>
	);
}
