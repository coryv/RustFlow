import React, { memo } from 'react';
import { Handle, Position, NodeProps } from 'reactflow';
import { Cpu } from 'lucide-react';
import clsx from 'clsx';

const AgentNode = ({ data, selected }: NodeProps) => {
    return (
        <div className={clsx(
            "shadow-md rounded-md bg-gray-900 border-2 min-w-[200px]",
            selected ? "border-purple-500" : "border-gray-700"
        )}>
            {/* Main Workflow Input */}
            <Handle
                type="target"
                position={Position.Left}
                id="workflow-in"
                className="w-3 h-3 !bg-gray-400 !border-2 !border-gray-800 top-1/2"
            />

            {/* Header */}
            <div className="px-4 py-3 border-b border-gray-800 flex items-center bg-gray-800 rounded-t-md">
                <div className="rounded-full w-8 h-8 flex items-center justify-center bg-purple-900/50 text-purple-400 mr-3">
                    <Cpu size={18} />
                </div>
                <div>
                    <div className="text-sm font-bold text-gray-200">AI Agent</div>
                    <div className="text-xs text-gray-500">LLM Processor</div>
                </div>
            </div>

            {/* Body / Slots */}
            <div className="p-4 space-y-4">
                {/* Model Slot */}
                <div className="relative bg-gray-950/50 rounded p-2 border border-gray-800 border-dashed">
                    <div className="text-xs text-gray-500 uppercase font-semibold mb-1">Model</div>
                    <div className="text-xs text-gray-600 italic">Connect a model...</div>

                    {/* Slot Handle */}
                    <Handle
                        type="target"
                        position={Position.Left}
                        id="model-slot"
                        className="w-3 h-3 !bg-purple-500 !border-2 !border-gray-900 !rounded-none"
                        style={{ left: -8, top: '50%' }}
                    />
                </div>
            </div>

            {/* Main Workflow Output */}
            <Handle
                type="source"
                position={Position.Right}
                id="workflow-out"
                className="w-3 h-3 !bg-gray-400 !border-2 !border-gray-800 top-1/2"
            />
        </div>
    );
};

export default memo(AgentNode);
