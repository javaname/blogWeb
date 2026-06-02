import { Navigate, Route, Routes } from 'react-router-dom';
import AppShell from './components/AppShell';
import ProtectedRoute from './components/ProtectedRoute';
import ArticleEdit from './pages/ArticleEdit';
import Categories from './pages/Categories';
import Comments from './pages/Comments';
import Dashboard from './pages/Dashboard';
import Login from './pages/Login';
import Analytics from './pages/Analytics';
import Media from './pages/Media';
import Posts from './pages/Posts';
import Settings from './pages/Settings';
import Users from './pages/Users';

export default function App() {
  return (
    <Routes>
      <Route path="/login" element={<Login />} />
      <Route
        path="/"
        element={
          <ProtectedRoute>
            <AppShell />
          </ProtectedRoute>
        }
      >
        <Route index element={<Navigate to="/dashboard" replace />} />
        <Route path="dashboard" element={<Dashboard />} />
        <Route path="posts" element={<Posts />} />
        <Route path="articles/new" element={<ArticleEdit />} />
        <Route path="articles/:id" element={<ArticleEdit />} />
        <Route path="categories" element={<Categories />} />
        <Route path="comments" element={<Comments />} />
        <Route path="media" element={<Media />} />
        <Route path="users" element={<Users />} />
        <Route path="analytics" element={<Analytics />} />
        <Route path="settings" element={<Settings />} />
      </Route>
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}
