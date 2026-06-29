import React from 'react';
import { createRoot } from 'react-dom/client';
import { App } from './App';
import { AdminAuthProvider } from './components/admin/AuthContext';
import { AdminToastProvider } from './components/admin/AdminToastProvider';
import './styles/index.css';

createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <AdminToastProvider>
      <AdminAuthProvider>
        <App />
      </AdminAuthProvider>
    </AdminToastProvider>
  </React.StrictMode>
);
