import { createBrowserRouter, Navigate } from "react-router-dom";
import { AppShell } from "./components/AppShell";
import { LoginPage } from "./views/LoginPage";
import { RegisterPage } from "./views/RegisterPage";
import { OrgsPage } from "./views/OrgsPage";
import { OrgAppPage } from "./views/OrgAppPage";
import { ChannelPage } from "./views/ChannelPage";
import { VoiceRoomPage } from "./views/VoiceRoomPage";
import { AdminPage } from "./views/AdminPage";

export const router = createBrowserRouter([
  {
    element: <AppShell />,
    children: [
      { path: "/", element: <Navigate to="/orgs" replace /> },
      { path: "/login", element: <LoginPage /> },
      { path: "/register", element: <RegisterPage /> },
      { path: "/orgs", element: <OrgsPage /> },
      { path: "/app/:org_slug", element: <OrgAppPage /> },
      { path: "/app/:org_slug/channels/:channel_id", element: <ChannelPage /> },
      { path: "/app/:org_slug/voice/:room_id", element: <VoiceRoomPage /> },
      { path: "/admin/:org_slug", element: <AdminPage /> },
    ],
  },
]);

