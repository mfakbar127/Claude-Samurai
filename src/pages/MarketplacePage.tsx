import { PackageIcon } from "lucide-react";
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
import { toast } from "sonner";
import {
	type KnownMarketplaces,
	type KnownMarketplace,
	useKnownMarketplaces,
} from "@/lib/query";

function getErrorMessage(error: unknown): string {
	if (error instanceof Error) {
		return error.message;
	}
	return String(error);
}

function MarketplaceHeader() {
	const { t } = useTranslation();

	return (
		<div
			className="flex items-center justify-between sticky top-0 z-10 border-b p-3 bg-background"
			data-tauri-drag-region
		>
			<div data-tauri-drag-region>
				<h3 className="font-bold" data-tauri-drag-region>
					{t("marketplace.title", { defaultValue: "Marketplace" })}
				</h3>
				<p
					className="text-sm text-muted-foreground"
					data-tauri-drag-region
				>
					{t("marketplace.description", {
						defaultValue:
							"View and manage Claude marketplace sources available on this system.",
					})}
				</p>
			</div>
		</div>
	);
}

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

function MarketplacePageContent() {
	const { t } = useTranslation();
	const {
		data: marketplaces,
		isLoading,
		error,
	} = useKnownMarketplaces();

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
					{t("marketplace.error", { error: getErrorMessage(error) })}
				</div>
			</div>
		);
	}

	const entries: [string, KnownMarketplace][] = Object.entries(
		(marketplaces ?? {}) as KnownMarketplaces,
	);

	if (entries.length === 0) {
		return (
			<div>
				<MarketplaceHeader />
				<div className="text-center text-muted-foreground py-8">
					{t("marketplace.empty", {
						defaultValue: "No installed marketplaces found.",
					})}
				</div>
			</div>
		);
	}

	const sortedEntries = entries.slice().sort(([aKey], [bKey]) =>
		aKey.localeCompare(bKey),
	);

	return (
		<div>
			<MarketplaceHeader />
			<div>
				<ScrollArea className="h-full">
					<div>
						<Accordion type="multiple">
							{sortedEntries.map(([key, value]) => (
								<AccordionItem
									key={key}
									value={key}
									className="bg-card"
								>
									<AccordionTrigger className="hover:no-underline px-4 py-2 bg-card hover:bg-accent duration-150">
										<div className="flex items-center justify-between gap-2 w-full">
											<div className="flex items-center gap-2 flex-wrap">
												<PackageIcon size={12} />
												<span className="font-medium">{key}</span>
												<Badge variant="secondary">
													{value.source.source}
												</Badge>
											</div>
											<div className="flex items-center gap-2">
												<span className="text-xs text-muted-foreground">
													{value.lastUpdated}
												</span>
											</div>
										</div>
									</AccordionTrigger>
									<AccordionContent className="pb-3">
										<div className="px-3 pt-3 space-y-3">
											<div className="space-y-1 text-sm text-muted-foreground">
												<div>
													<span className="font-medium">
														{t("marketplace.repo", {
															defaultValue: "Repository:",
														})}{" "}
													</span>
													<span className="font-mono text-xs">
														{value.source.repo}
													</span>
												</div>
												<div>
													<span className="font-medium">
														{t("marketplace.installLocation", {
															defaultValue: "Install location:",
														})}{" "}
													</span>
													<span className="font-mono text-xs">
														{value.installLocation}
													</span>
												</div>
												<div>
													<span className="font-medium">
														{t("marketplace.lastUpdated", {
															defaultValue: "Last updated:",
														})}{" "}
													</span>
													<span className="text-xs">
														{value.lastUpdated}
													</span>
												</div>
											</div>

											<div className="bg-card">
												<p className="text-[11px] text-muted-foreground">
													{t("marketplace.updateInstruction", {
														defaultValue:
															"Run this in your terminal to update:",
													})}
												</p>
												<div className="flex items-center gap-2 mt-1">
													<div className="rounded-md bg-muted px-3 py-2 font-mono text-xs break-all flex-1 select-text">
														{`claude plugin marketplace update ${key}`}
													</div>
													<Button
														variant="outline"
														size="sm"
														onClick={(e) => {
															e.stopPropagation();
															void copyToClipboard(
																`claude plugin marketplace update ${key}`,
															);
														}}
													>
														Copy
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
			</div>
		</div>
	);
}

export function MarketplacePage() {
	const { t } = useTranslation();

	return (
		<Suspense
			fallback={
				<div className="flex items-center justify-center min-h-screen">
					<div className="text-center">{t("loading")}</div>
				</div>
			}
		>
			<MarketplacePageContent />
		</Suspense>
	);
}

