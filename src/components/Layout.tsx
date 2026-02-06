import {
	ActivityIcon,
	BellIcon,
	BotIcon,
	BrainIcon,
	CpuIcon,
	FileJsonIcon,
	FolderIcon,
	PackageIcon,
	SettingsIcon,
	SparklesIcon,
	TerminalIcon,
	ShieldCheckIcon,
} from "lucide-react";
import type { CSSProperties } from "react";
import { useTranslation } from "react-i18next";
import { NavLink, Outlet, useLocation } from "react-router-dom";
import { cn, isMacOS } from "../lib/utils";
import { UpdateButton } from "./UpdateButton";
import { ScrollArea } from "./ui/scroll-area";

const macosDragRegionStyle = {
	WebkitUserSelect: "none",
	WebkitAppRegion: "drag",
} as CSSProperties;

function MacosDragRegionSpacer(props: { className: string }) {
	if (!isMacOS) {
		return null;
	}

	return (
		<div
			data-tauri-drag-region
			className={props.className}
			style={macosDragRegionStyle}
		/>
	);
}

export function Layout() {
	const { t } = useTranslation();
	const location = useLocation();
	const isProjectsRoute = location.pathname.startsWith("/projects");

	const navSections = [
		{
			id: "core",
			label: t("navigation.section.core", { defaultValue: "Core" }),
			items: [
				{
					to: "/",
					icon: FileJsonIcon,
					label: t("navigation.configurations"),
				},
				{
					to: "/projects",
					icon: FolderIcon,
					label: t("navigation.projects"),
				},
				{
					to: "/mcp",
					icon: CpuIcon,
					label: t("navigation.mcp"),
				},
				{
					to: "/memory",
					icon: BrainIcon,
					label: t("navigation.memory"),
				},
				{
					to: "/plugins",
					icon: PackageIcon,
					label: t("navigation.plugins"),
				},
			],
		},
		{
			id: "automation",
			label: t("navigation.section.automation", { defaultValue: "Automation" }),
			items: [
				{
					to: "/agents",
					icon: BotIcon,
					label: "Agents",
				},
				{
					to: "/commands",
					icon: TerminalIcon,
					label: t("navigation.commands"),
				},
				{
					to: "/skills",
					icon: SparklesIcon,
					label: t("navigation.skills"),
				},
				{
					to: "/hooks",
					icon: CpuIcon,
					label: t("hooks.title"),
				},
			],
		},
		{
			id: "system",
			label: t("navigation.section.system", { defaultValue: "System" }),
			items: [
				{
					to: "/security-packs",
					icon: ShieldCheckIcon,
					label: "Security Packs",
				},
				{
					to: "/notification",
					icon: BellIcon,
					label: t("navigation.notifications"),
				},
				{
					to: "/usage",
					icon: ActivityIcon,
					label: t("navigation.usage"),
				},
				{
					to: "/settings",
					icon: SettingsIcon,
					label: t("navigation.settings"),
				},
			],
		},
	] as const;

	return (
		<div className="min-h-screen bg-background flex flex-col">
			{/* Custom Title Bar - Draggable Region with traffic lights space (macOS only) */}
			<MacosDragRegionSpacer className="" />

			<div className="flex flex-1 overflow-hidden ">
				<nav
					className="w-[200px] bg-background border-r flex flex-col"
					aria-label={t("navigation.primary", {
						defaultValue: "Primary navigation",
					})}
					data-tauri-drag-region
				>
					<MacosDragRegionSpacer className="h-10" />
					<div className="flex flex-col flex-1 justify-between">
						<div className="px-3 pt-3 space-y-4">
							{navSections.map((section) => (
								<div key={section.id} className="space-y-2">
									<p
										className="px-1 text-[11px] font-medium uppercase tracking-wide text-muted-foreground/80"
										role="heading"
										aria-level={2}
									>
										{section.label}
									</p>
									<ul className="space-y-1">
										{section.items.map((link) => (
											<li key={link.to}>
												<NavLink
													to={link.to}
													className={({ isActive }) =>
														cn(
															"flex items-center gap-2 px-3 py-2 rounded-xl cursor-default select-none text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 focus-visible:ring-offset-background",
															{
																"bg-primary text-primary-foreground":
																	isActive,
																"hover:bg-accent hover:text-accent-foreground":
																	!isActive,
															},
														)
													}
												>
													<link.icon size={14} />
													{link.label}
												</NavLink>
											</li>
										))}
									</ul>
								</div>
							))}
						</div>

						<div className="space-y-2 px-3 pb-3 pt-4 border-t">
							<UpdateButton />
						</div>
					</div>
				</nav>
				{isProjectsRoute ? (
					<main
						className="flex-1 h-screen overflow-hidden"
						data-tauri-drag-region
					>
						<Outlet />
					</main>
				) : (
					<ScrollArea className="flex-1 h-screen [&>div>div]:!block">
						<main className="" data-tauri-drag-region>
							<Outlet />
						</main>
					</ScrollArea>
				)}
			</div>
		</div>
	);
}
