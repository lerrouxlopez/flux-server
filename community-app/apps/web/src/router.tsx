import { createBrowserRouter, Navigate } from "react-router-dom";
import { AppShell } from "./components/AppShell";
import { RequireAuth, RequireGuest } from "./components/AuthGate";
import { LoginPage } from "./views/LoginPage";
import { RegisterPage } from "./views/RegisterPage";
import { OrganizationGalleryPage } from "./features/orgs/pages/OrganizationGalleryPage";
import { OrgAppPage } from "./views/OrgAppPage";
import { ChannelPage } from "./views/ChannelPage";
import { VoiceRoomPage } from "./views/VoiceRoomPage";
import { AdminPage } from "./views/AdminPage";
import { FriendsPage } from "./views/FriendsPage";
import { ProfilePage } from "./views/ProfilePage";
import { NotificationSettingsPage } from "./views/NotificationSettingsPage";

export const router = createBrowserRouter([
  {
    element: <AppShell />,
    children: [
      { path: "/", element: <Navigate to="/orgs" replace /> },
      { path: "/login", element: <RequireGuest><LoginPage /></RequireGuest> },
      { path: "/register", element: <RequireGuest><RegisterPage /></RequireGuest> },
      { path: "/orgs", element: <RequireAuth><OrganizationGalleryPage /></RequireAuth> },
      { path: "/app/:org_slug", element: <RequireAuth><OrgAppPage /></RequireAuth> },
      { path: "/app/:org_slug/channels/:channel_id", element: <RequireAuth><ChannelPage /></RequireAuth> },
      { path: "/app/:org_slug/voice/:room_id", element: <RequireAuth><VoiceRoomPage /></RequireAuth> },
      { path: "/app/:org_slug/friends", element: <RequireAuth><FriendsPage /></RequireAuth> },
      { path: "/app/:org_slug/settings/notifications", element: <RequireAuth><NotificationSettingsPage /></RequireAuth> },
      { path: "/profile", element: <RequireAuth><ProfilePage /></RequireAuth> },
      { path: "/admin/:org_slug", element: <RequireAuth><AdminPage /></RequireAuth> },
    ],
  },
]);
