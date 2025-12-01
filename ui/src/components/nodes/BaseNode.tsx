import React, { memo } from 'react';
import { Handle, Position, NodeProps } from 'reactflow';
// import { Play, Clock, Globe, Database, Code, ArrowRightLeft, FileJson, Terminal, FileText, Cpu } from 'lucide-react';
// import clsx from 'clsx';

const icons: Record<string, string> = {
    manual_trigger: '▶️',
    time_trigger: '⏰',
    webhook_trigger: '🌐',
    http_request: '🌐',
    code: '💻',
    set_data: '💾',
    console_output: '📟',
    router: '🔀',
    notion_create_page: '📄',
    html_extract: '🔍',
    join: '🔗',
    union: '∪',
    file_source: '📁',
    agent: '🤖',
};

const BaseNode = ({ data, type, selected }: any) => {
    const Icon = icons[type] || '📦';

    // Determine if this is a trigger (no input) or action (input + output)
    const triggerTypes = ['manual_trigger', 'time_trigger', 'webhook_trigger'];
    const isTrigger = triggerTypes.includes(type) || type.includes('trigger');

    return (
        <div className={`px-4 py-2 shadow-md rounded-md bg-gray-800 border-2 min-w-[150px] ${selected ? "border-blue-500" : "border-gray-700"}`}>
            {!isTrigger && (
                <Handle
                    type="target"
                    position={Position.Left}
                    className="w-3 h-3 !bg-gray-400 !border-2 !border-gray-800"
                />
            )}

            <div className="flex items-center">
                <div className="rounded-full w-8 h-8 flex items-center justify-center bg-gray-700 text-gray-300 mr-3">
                    <span style={{ fontSize: '16px' }}>{Icon}</span>
                </div>
                <div className="ml-1">
                    <div className="text-sm font-bold text-gray-200">{data.label}</div>
                    {data.description && <div className="text-xs text-gray-500">{data.description}</div>}
                </div>
            </div>

            <Handle
                type="source"
                position={Position.Right}
                className="w-3 h-3 !bg-gray-400 !border-2 !border-gray-800"
            />
        </div>
    );
};

export default memo(BaseNode);
