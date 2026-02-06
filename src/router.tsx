import type React from "react";
import { createBrowserRouter, RouterProvider } from "react-router-dom";
import { Layout } from "./components/Layout";
import { RouteWrapper } from "./components/RouteWrapper";
import { AgentsPage } from "./pages/AgentsPage";
import { CommandsPage } from "./pages/CommandsPage";
import { ConfigEditorPage } from "./pages/ConfigEditorPage";
import { ConfigSwitcherPage } from "./pages/ConfigSwitcherPage";
import { HooksPage } from "./pages/HooksPage";
import { MCPPage } from "./pages/MCPPage";
import { MemoryPage } from "./pages/MemoryPage";
import { SkillsPage } from "./pages/SkillsPage";
import { NotificationPage } from "./pages/NotificationPage";
import { PluginsPage } from "./pages/PluginsPage";
import { SecurityPacksPage } from "./pages/SecurityPacksPage";
import { Detail } from "./pages/projects/Detail";
import { ProjectsLayout } from "./pages/projects/Layout";
import { List } from "./pages/projects/List";
import { SettingsPage } from "./pages/SettingsPage";
import { UsagePage } from "./pages/UsagePage";

function wrapRoute(element: React.ReactNode): React.ReactNode {
	return <RouteWrapper>{element}</RouteWrapper>;
}

const router = createBrowserRouter([
	{
		path: "/",
		element: wrapRoute(<Layout />),
		children: [
			{
				index: true,
				element: wrapRoute(<ConfigSwitcherPage />),
			},
			{
				path: "edit/:storeId",
				element: wrapRoute(<ConfigEditorPage />),
			},
			{
				path: "settings",
				element: wrapRoute(<SettingsPage />),
			},
			{
				path: "mcp",
				element: wrapRoute(<MCPPage />),
			},
			{
				path: "hooks",
				element: wrapRoute(<HooksPage />),
			},
			{
				path: "agents",
				element: wrapRoute(<AgentsPage />),
			},
			{
				path: "usage",
				element: wrapRoute(<UsagePage />),
			},
			{
				path: "memory",
				element: wrapRoute(<MemoryPage />),
			},
			{
				path: "notification",
				element: wrapRoute(<NotificationPage />),
			},
			{
				path: "commands",
				element: wrapRoute(<CommandsPage />),
			},
			{
				path: "skills",
				element: wrapRoute(<SkillsPage />),
			},
			{
				path: "plugins",
				element: wrapRoute(<PluginsPage />),
			},
			{
				path: "security-packs",
				element: wrapRoute(<SecurityPacksPage />),
			},
			{
				path: "projects",
				element: wrapRoute(<ProjectsLayout />),
				children: [
					{
						index: true,
						element: wrapRoute(<List />),
					},
					{
						path: ":path",
						element: wrapRoute(<Detail />),
					},
				],
			},
		],
	},
]);

export function Router() {
	return <RouterProvider router={router} />;
}
