import React, { useRef } from 'react';
import { Play, Download, Upload, FileCode } from 'lucide-react';

interface ToolbarProps {
    onRun: () => void;
    onExport: () => void;
    onImport: (content: string) => void;
    wasmReady: boolean;
}

export default function Toolbar({ onRun, onExport, onImport, wasmReady }: ToolbarProps) {
    const fileInputRef = useRef<HTMLInputElement>(null);

    const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        const file = e.target.files?.[0];
        if (file) {
            const reader = new FileReader();
            reader.onload = (event) => {
                const content = event.target?.result as string;
                onImport(content);
            };
            reader.readAsText(file);
        }
    };

    return (
        <div className="h-14 bg-gray-900 border-b border-gray-800 flex items-center justify-between px-4">
            <div className="flex items-center gap-2">
                <div className="w-8 h-8 bg-blue-600 rounded flex items-center justify-center">
                    <FileCode className="text-white" size={20} />
                </div>
                <h1 className="text-lg font-bold text-white">RustFlow</h1>
            </div>

            <div className="flex items-center gap-2">
                <button
                    onClick={() => fileInputRef.current?.click()}
                    className="flex items-center gap-2 px-3 py-1.5 text-sm font-medium text-gray-300 hover:text-white hover:bg-gray-800 rounded transition-colors"
                >
                    <Upload size={16} />
                    Import
                </button>
                <input
                    type="file"
                    ref={fileInputRef}
                    onChange={handleFileChange}
                    className="hidden"
                    accept=".yaml,.yml,.json"
                />

                <button
                    onClick={onExport}
                    className="flex items-center gap-2 px-3 py-1.5 text-sm font-medium text-gray-300 hover:text-white hover:bg-gray-800 rounded transition-colors"
                >
                    <Download size={16} />
                    Export
                </button>

                <div className="h-6 w-px bg-gray-700 mx-2" />

                <button
                    onClick={onRun}
                    disabled={!wasmReady}
                    className={`flex items-center gap-2 px-4 py-1.5 text-sm font-medium text-white rounded transition-colors ${wasmReady ? 'bg-blue-600 hover:bg-blue-700' : 'bg-gray-700 cursor-not-allowed opacity-50'
                        }`}
                >
                    <Play size={16} />
                    Run
                </button>
            </div>
        </div>
    );
}
