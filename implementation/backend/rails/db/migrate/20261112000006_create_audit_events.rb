# frozen_string_literal: true

class CreateAuditEvents < ActiveRecord::Migration[7.2]
  def change
    create_table :audit_events do |t|
      t.string :action, null: false
      t.string :resource_type, null: false
      t.bigint :resource_id
      t.jsonb :metadata, default: {}
      t.string :actor_id
      t.timestamps
    end
    add_index :audit_events, [:resource_type, :resource_id]
  end
end
