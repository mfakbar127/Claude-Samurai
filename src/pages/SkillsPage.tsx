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
import { ScrollArea } from "@/components/ui/scroll-area";
import { codeMirrorBasicSetup, markdownExtensions } from "@/lib/codemirror-config";
import { useClaudeSkills } from "@/lib/query";
import { useCodeMirrorTheme } from "@/lib/use-codemirror-theme";

function SkillsPageContent() {
	const { t } = useTranslation();
	const { data: skills, isLoading, error } = useClaudeSkills();
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

	const skillList = skills ?? [];

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
											<div className="flex items-center gap-2 flex-wrap">
												<SparklesIcon size={12} />
												<span className="font-medium">{skill.name}</span>
											</div>
										</AccordionTrigger>
										<AccordionContent className="pb-3">
											<div className="px-3 pt-3 space-y-3">
												<div className="rounded-lg overflow-hidden border">
													<CodeMirror
														value={skill.content}
														height="280px"
														theme={codeMirrorTheme}
														placeholder={t("skills.contentPlaceholder")}
														extensions={markdownExtensions}
														basicSetup={codeMirrorBasicSetup}
														editable={false}
													/>
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
