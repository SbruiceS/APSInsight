# frozen_string_literal: true

class CreateMemberships < ActiveRecord::Migration[7.2]
  def change
    create_table :memberships do |t|
      t.references :user, null: false, foreign_key: true
      t.references :organization, null: false, foreign_key: true
      t.references :project, foreign_key: true
      t.references :role, null: false, foreign_key: true
      t.timestamps
    end
    add_index :memberships, [:user_id, :organization_id, :project_id], unique: true, name: "index_memberships_on_user_org_project"
  end
end
