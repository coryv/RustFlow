import React, { memo } from 'react';
import { Handle, Position, NodeProps } from 'reactflow';
import { BrainCircuit } from 'lucide-react';
import clsx from 'clsx';

const ModelNode = ({ data, selected }: NodeProps) => {
    return (
        <div className={clsx(
            "px-3 py-2 shadow-md rounded-md bg-purple-900/20 border-2 min-w-[140px]",
            selected ? "border-purple-400" : "border-purple-900/50"
        )}>
            <div className="flex items-center">
                <div className="mr-2 text-purple-400">
                    <BrainCircuit size={14} />
                </div>
                <div className="text-xs font-bold text-purple-200">{data.label}</div>
            </div>

            {/* Output Handle Only - Connects to Agent Slot */}
            <Handle
                type="source"
                position={Position.Right}
                className="w-3 h-3 !bg-purple-500 !border-2 !border-gray-900 !rounded-none"
            />
        </div>
    );
};

export default memo(ModelNode);
