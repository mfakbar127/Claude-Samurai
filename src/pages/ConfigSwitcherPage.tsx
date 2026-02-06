import { Kimi, Minimax, ZAI } from "@lobehub/icons";
import { EllipsisVerticalIcon, PencilLineIcon, PlusIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { GLMDialog } from "@/components/GLMBanner";
import { KimiDialog } from "@/components/KimiDialog";
import { MiniMaxDialog } from "@/components/MiniMaxDialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ButtonGroup } from "@/components/ui/button-group";
import {
	AlertDialog,
	AlertDialogAction,
	AlertDialogCancel,
	AlertDialogContent,
	AlertDialogDescription,
	AlertDialogFooter,
	AlertDialogHeader,
	AlertDialogTitle,
	AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
	useCreateConfig,
	useDeleteConfig,
	useResetToOriginalConfig,
	useSetCurrentConfig,
	useStores,
} from "../lib/query";

export function ConfigSwitcherPage() {
	return (
		<section>
			<ConfigStores />
		</section>
	);
}

function ConfigStores() {
	const { t } = useTranslation();
	const { data: stores } = useStores();
	const setCurrentStoreMutation = useSetCurrentConfig();
	const resetToOriginalMutation = useResetToOriginalConfig();
	const navigate = useNavigate();

	const isOriginalConfigActive = !stores.some((store) => store.using);
	const activeStore = stores.find((store) => store.using) ?? null;
	const otherStores = activeStore
		? stores.filter((store) => store.id !== activeStore.id)
		: stores;

	const handleStoreClick = (storeId: string, isCurrentStore: boolean) => {
		if (!isCurrentStore) {
			setCurrentStoreMutation.mutate(storeId);
		}
	};

	const handleOriginalConfigClick = () => {
		if (!isOriginalConfigActive) {
			resetToOriginalMutation.mutate();
		}
	};

	const createStoreMutation = useCreateConfig();
	const deleteStoreMutation = useDeleteConfig();

	const onCreateStore = async () => {
		const store = await createStoreMutation.mutateAsync({
			title: t("configSwitcher.newConfig"),
			settings: {},
		});
		if (store) {
			navigate(`/edit/${store.id}`);
		}
	};

	if (stores.length === 0) {
		return (
			<div className="flex h-[calc(100vh-64px)] items-center justify-center px-6">
				<div className="flex max-w-md flex-col items-center gap-3 text-center">
					<h1 className="text-xl font-semibold">
						{t("configSwitcher.emptyTitle", {
							defaultValue: "No configurations yet",
						})}
					</h1>
					<p className="text-sm text-muted-foreground">
						{t("configSwitcher.description")}
					</p>

					<ButtonGroup className="mt-2">
						<Button
							onClick={onCreateStore}
							size="sm"
							disabled={createStoreMutation.isPending}
						>
							<PlusIcon size={14} />
							{t("configSwitcher.createConfig")}
						</Button>
						<DropdownMenu>
							<DropdownMenuTrigger asChild>
								<Button
									variant="outline"
									size="sm"
									disabled={createStoreMutation.isPending}
								>
									<EllipsisVerticalIcon size={14} />
								</Button>
							</DropdownMenuTrigger>
							<DropdownMenuContent align="end">
								<GLMDialog
									trigger={
										<DropdownMenuItem onSelect={(e) => e.preventDefault()}>
											<ZAI />
											{t("glm.useZhipuGlm")}
										</DropdownMenuItem>
									}
								/>
								<MiniMaxDialog
									trigger={
										<DropdownMenuItem onSelect={(e) => e.preventDefault()}>
											<Minimax />
											{t("minimax.useMiniMax")}
										</DropdownMenuItem>
									}
								/>
								<KimiDialog
									trigger={
										<DropdownMenuItem onSelect={(e) => e.preventDefault()}>
											<Kimi />
											{t("kimi.useKimi")}
										</DropdownMenuItem>
									}
								/>
							</DropdownMenuContent>
						</DropdownMenu>
					</ButtonGroup>

					<p className="mt-4 text-xs text-muted-foreground">
						{t("configSwitcher.emptyHelper", {
							defaultValue:
								"Start from a template like Zhipu GLM, MiniMax, or Kimi, or create a config from scratch.",
						})}
					</p>
				</div>
			</div>
		);
	}

	return (
		<div className="space-y-6 p-4">
			<header className="flex items-start justify-between gap-4 border-b pb-3">
				<div>
					<h1 className="text-lg font-semibold">
						{t("configSwitcher.title")}
					</h1>
					<p className="mt-1 text-sm text-muted-foreground">
						{t("configSwitcher.description")}
					</p>
				</div>
				<ButtonGroup>
					<Button
						onClick={onCreateStore}
						size="sm"
						disabled={createStoreMutation.isPending}
					>
						<PlusIcon size={14} />
						{t("configSwitcher.createConfig")}
					</Button>
					<DropdownMenu>
						<DropdownMenuTrigger asChild>
							<Button
								variant="outline"
								size="sm"
								disabled={createStoreMutation.isPending}
							>
								<EllipsisVerticalIcon size={14} />
							</Button>
						</DropdownMenuTrigger>
						<DropdownMenuContent align="end">
							<GLMDialog
								trigger={
									<DropdownMenuItem onSelect={(e) => e.preventDefault()}>
										<ZAI />
										{t("glm.useZhipuGlm")}
									</DropdownMenuItem>
								}
							/>
							<MiniMaxDialog
								trigger={
									<DropdownMenuItem onSelect={(e) => e.preventDefault()}>
										<Minimax />
										{t("minimax.useMiniMax")}
									</DropdownMenuItem>
								}
							/>
							<KimiDialog
								trigger={
									<DropdownMenuItem onSelect={(e) => e.preventDefault()}>
										<Kimi />
										{t("kimi.useKimi")}
									</DropdownMenuItem>
								}
							/>
						</DropdownMenuContent>
					</DropdownMenu>
				</ButtonGroup>
			</header>

			<section className="space-y-3">
				<div className="flex items-center justify-between">
					<h2 className="text-sm font-medium">
						{t("configSwitcher.activeConfigHeading", {
							defaultValue: "Active configuration",
						})}
					</h2>
					<div className="flex items-center gap-2">
						<Badge variant="success">
							{t("configSwitcher.activeBadge", { defaultValue: "Active" })}
						</Badge>
						<Button
							variant="ghost"
							size="sm"
							className="h-7 px-2 text-xs text-muted-foreground hover:text-foreground"
							onClick={() => navigate("/settings")}
						>
							{t("configSwitcher.viewHistory", {
								defaultValue: "Backups & history",
							})}
						</Button>
					</div>
				</div>

				<div className="rounded-xl border bg-card p-4">
					{activeStore ? (
						<div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
							<div className="space-y-1">
								<div className="flex items-center gap-2">
									<h3 className="font-medium">{activeStore.title}</h3>
									<Badge
										variant="secondary"
										className="text-[11px] font-medium uppercase"
									>
										{t("configSwitcher.tagDailyDriver", {
											defaultValue: "Daily driver",
										})}
									</Badge>
								</div>
								{activeStore.settings.env?.ANTHROPIC_BASE_URL && (
									<p className="text-xs text-muted-foreground">
										{activeStore.settings.env.ANTHROPIC_BASE_URL}
									</p>
								)}
							</div>
							<div className="flex gap-2">
								<Button
									variant="outline"
									size="sm"
									onClick={() => navigate(`/edit/${activeStore.id}`)}
								>
									<PencilLineIcon size={14} className="mr-1" />
									{t("configSwitcher.edit", { defaultValue: "Edit" })}
								</Button>
							</div>
						</div>
					) : (
						<div className="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
							<div className="space-y-1">
								<div className="flex items-center gap-2">
									<h3 className="font-medium">
										{t("configSwitcher.originalConfig", {
											defaultValue: "Original Claude config",
										})}
									</h3>
									<Badge
										variant="secondary"
										className="text-[11px] font-medium uppercase"
									>
										{t("configSwitcher.tagDefault", {
											defaultValue: "Default",
										})}
									</Badge>
								</div>
								<p className="text-xs text-muted-foreground">
									{t("configSwitcher.originalConfigDescription")}
								</p>
							</div>
							<Button
								variant="outline"
								size="sm"
								onClick={handleOriginalConfigClick}
								disabled={
									isOriginalConfigActive || resetToOriginalMutation.isPending
								}
							>
								{t("configSwitcher.restoreOriginal", {
									defaultValue: "Restore original",
								})}
							</Button>
						</div>
					)}
				</div>
			</section>

			<section className="space-y-3">
				<div className="flex items-center justify-between">
					<h2 className="text-sm font-medium">
						{t("configSwitcher.otherConfigsHeading", {
							defaultValue: "Other configurations",
						})}
					</h2>
					<p className="text-xs text-muted-foreground">
						{t("configSwitcher.otherConfigsHelper", {
							defaultValue:
								"Use these for experiments, different providers, or project-specific setups.",
						})}
					</p>
				</div>

				{otherStores.length === 0 ? (
					<p className="text-sm text-muted-foreground">
						{t("configSwitcher.noOtherConfigs", {
							defaultValue: "You only have the active configuration right now.",
						})}
					</p>
				) : (
					<div className="grid gap-3 md:grid-cols-2 lg:grid-cols-3">
						{otherStores.map((store) => (
							<div
								key={store.id}
								className="flex flex-col justify-between rounded-xl border bg-card p-3 transition-colors hover:border-primary/60"
							>
								<div className="space-y-1">
									<div className="flex items-center justify-between gap-2">
										<h3 className="truncate text-sm font-medium">
											{store.title}
										</h3>
									</div>
									{store.settings.env?.ANTHROPIC_BASE_URL && (
										<p
											className="truncate text-xs text-muted-foreground"
											title={store.settings.env.ANTHROPIC_BASE_URL}
										>
											{store.settings.env.ANTHROPIC_BASE_URL}
										</p>
									)}
								</div>

								<div className="mt-3 flex items-center justify-between gap-2">
									<Button
										variant="outline"
										size="sm"
										className="h-8 px-2 text-xs"
										onClick={() => navigate(`/edit/${store.id}`)}
									>
										<PencilLineIcon size={12} className="mr-1" />
										{t("configSwitcher.edit", { defaultValue: "Edit" })}
									</Button>
									<div className="flex gap-1">
										<Button
											variant="outline"
											size="sm"
											className="h-8 px-2 text-xs"
											onClick={() =>
												handleStoreClick(store.id, store.using)
											}
											disabled={setCurrentStoreMutation.isPending}
										>
											{t("configSwitcher.activate", {
												defaultValue: "Activate",
											})}
										</Button>
										<Button
											variant="outline"
											size="sm"
											className="h-8 px-2 text-xs"
											onClick={() =>
												createStoreMutation.mutate({
													title: `${store.title} (copy)`,
													settings: store.settings,
												})
											}
											disabled={createStoreMutation.isPending}
										>
											{t("configSwitcher.duplicate", {
												defaultValue: "Duplicate",
											})}
										</Button>
										<AlertDialog>
											<AlertDialogTrigger asChild>
												<Button
													variant="outline"
													size="sm"
													className="h-8 px-2 text-xs"
													disabled={deleteStoreMutation.isPending}
												>
													{t("configSwitcher.delete", {
														defaultValue: "Delete",
													})}
												</Button>
											</AlertDialogTrigger>
											<AlertDialogContent>
												<AlertDialogHeader>
													<AlertDialogTitle>
														{t("configSwitcher.deleteConfirmTitle", {
															defaultValue: "Delete configuration?",
														})}
													</AlertDialogTitle>
													<AlertDialogDescription>
														{t(
															"configSwitcher.deleteConfirmDescription",
															{
																defaultValue:
																	"This will permanently remove this configuration from Claude Samurai. This action cannot be undone.",
															},
														)}
													</AlertDialogDescription>
												</AlertDialogHeader>
												<AlertDialogFooter>
													<AlertDialogCancel disabled={deleteStoreMutation.isPending}>
														{t("common.cancel", {
															defaultValue: "Cancel",
														})}
													</AlertDialogCancel>
													<AlertDialogAction
														onClick={() =>
															deleteStoreMutation.mutate({
																storeId: store.id,
															})
														}
														className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
														disabled={deleteStoreMutation.isPending}
													>
														{t("configSwitcher.deleteConfirmAction", {
															defaultValue: "Delete",
														})}
													</AlertDialogAction>
												</AlertDialogFooter>
											</AlertDialogContent>
										</AlertDialog>
									</div>
								</div>
							</div>
						))}
					</div>
				)}
			</section>
		</div>
	);
}
